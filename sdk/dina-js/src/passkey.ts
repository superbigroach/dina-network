/**
 * PasskeyWallet — WebAuthn-based P-256 wallet for Dina Network user wallets.
 *
 * BROWSER ENVIRONMENT REQUIRED.
 * This module depends on `navigator.credentials` (Web Authentication API) and
 * must run in a browser or browser-like environment (e.g. a WebView). It will
 * NOT work in plain Node.js.
 *
 * For React Native, use `react-native-passkeys` (or `@daimo/expo-passkeys`)
 * in place of this module — the WebAuthn API is not available in the RN runtime.
 *
 * Architecture note:
 * Private keys for passkey wallets NEVER leave the device's secure enclave
 * (Secure Enclave on iOS/macOS, Strongbox/TEE on Android, TPM on Windows).
 * The DRC-111 smart-wallet contract stores only the P-256 public key and
 * verifies ECDSA signatures on-chain. There is no export path for the private
 * key — this is by design.
 */

/** Result returned from `PasskeyWallet.register()`. */
export interface PasskeyRegistration {
  /** The credential ID assigned by the authenticator. */
  credentialId: Uint8Array;
  /**
   * The P-256 public key in either:
   *   - compressed form (33 bytes, prefix 0x02 or 0x03), or
   *   - uncompressed form (65 bytes, prefix 0x04).
   * Store this in the DRC-111 `PasskeyCredential.public_key` field.
   */
  publicKey: Uint8Array;
}

/** Result returned from `PasskeyWallet.sign()`. */
export interface PasskeyAssertion {
  /** Raw authenticatorData bytes from the WebAuthn assertion response. */
  authenticatorData: Uint8Array;
  /** UTF-8 clientDataJSON bytes from the WebAuthn assertion response. */
  clientDataJSON: Uint8Array;
  /**
   * DER-encoded P-256/ECDSA signature over
   *   authenticatorData || SHA-256(clientDataJSON).
   * Pass this directly to the DRC-111 `execute_with_passkey` method.
   */
  signature: Uint8Array;
  /**
   * Authenticator sign counter. Must be strictly greater than the value
   * stored in the DRC-111 contract to prevent replay attacks.
   */
  counter: number;
}

/**
 * Extract the raw P-256 public key bytes from a CBOR-encoded COSE_Key
 * produced by `navigator.credentials.create()`.
 *
 * COSE_Key map for P-256 (RFC 8152 §13.1.1):
 *   1  (kty)  => 2  (EC2)
 *   3  (alg)  => -7 (ES256)
 *  -1  (crv)  => 1  (P-256)
 *  -2  (x)    => bstr  (32 bytes)
 *  -3  (y)    => bstr  (32 bytes)
 *
 * We return an uncompressed point (0x04 || x || y, 65 bytes) so callers
 * can use it directly with any P-256 library.
 */
function coseKeyToUncompressedPoint(coseKey: ArrayBuffer): Uint8Array {
  // Minimal CBOR decoder for the P-256 COSE_Key map.
  // This handles only the subset produced by WebAuthn authenticators.
  const bytes = new Uint8Array(coseKey);
  let offset = 0;

  function readByte(): number {
    if (offset >= bytes.length) throw new Error('PasskeyWallet: unexpected end of COSE_Key');
    return bytes[offset++];
  }

  function readUint(additionalInfo: number): number {
    if (additionalInfo < 24) return additionalInfo;
    if (additionalInfo === 24) return readByte();
    if (additionalInfo === 25) {
      const hi = readByte();
      const lo = readByte();
      return (hi << 8) | lo;
    }
    throw new Error(`PasskeyWallet: unsupported CBOR uint size ${additionalInfo}`);
  }

  function readItem(): unknown {
    const initial = readByte();
    const majorType = initial >> 5;
    const additionalInfo = initial & 0x1f;

    if (majorType === 0) {
      // Unsigned integer
      return readUint(additionalInfo);
    }
    if (majorType === 1) {
      // Negative integer: -1 - n
      return -(1 + readUint(additionalInfo));
    }
    if (majorType === 2) {
      // Byte string
      const len = readUint(additionalInfo);
      const slice = bytes.slice(offset, offset + len);
      offset += len;
      return slice;
    }
    if (majorType === 5) {
      // Map
      const count = readUint(additionalInfo);
      const map = new Map<number, unknown>();
      for (let i = 0; i < count; i++) {
        const key = readItem() as number;
        const value = readItem();
        map.set(key, value);
      }
      return map;
    }
    throw new Error(`PasskeyWallet: unsupported CBOR major type ${majorType}`);
  }

  const map = readItem() as Map<number, unknown>;
  const x = map.get(-2) as Uint8Array | undefined;
  const y = map.get(-3) as Uint8Array | undefined;
  if (!x || !y || x.length !== 32 || y.length !== 32) {
    throw new Error('PasskeyWallet: malformed P-256 COSE_Key — missing x or y coordinate');
  }

  // Return uncompressed point: 0x04 || x || y
  const point = new Uint8Array(65);
  point[0] = 0x04;
  point.set(x, 1);
  point.set(y, 33);
  return point;
}

/**
 * PasskeyWallet wraps the WebAuthn browser API to create and use
 * P-256 passkey credentials for Dina Network user wallets.
 *
 * Private keys NEVER leave the secure enclave. Only the public key
 * is stored on-chain in the DRC-111 smart wallet contract.
 */
export class PasskeyWallet {
  /**
   * Register a new passkey credential for a user.
   *
   * Calls `navigator.credentials.create()` which prompts the user to
   * authenticate with their device biometrics / PIN. The resulting
   * credential ID and P-256 public key should be stored in the DRC-111
   * contract via the `init` or an add-passkey method.
   *
   * @param rpId     - Relying Party domain, e.g. `"eltesoro.hn"`.
   *                   Must match the current page's effective domain.
   * @param userName - Human-readable user name or email shown in the
   *                   authenticator prompt (e.g. `"alice@eltesoro.hn"`).
   * @param userId   - Opaque user handle (max 64 bytes). A stable,
   *                   non-PII identifier for the account — e.g. a random
   *                   UUID encoded as bytes.
   * @returns `credentialId` and `publicKey` (uncompressed P-256 point, 65 bytes).
   */
  static async register(
    rpId: string,
    userName: string,
    userId: Uint8Array,
  ): Promise<PasskeyRegistration> {
    if (typeof navigator === 'undefined' || !navigator.credentials) {
      throw new Error(
        'PasskeyWallet: navigator.credentials is not available. ' +
          'This API requires a browser environment. ' +
          'For React Native, use react-native-passkeys instead.',
      );
    }

    const credential = await navigator.credentials.create({
      publicKey: {
        rp: {
          id: rpId,
          name: rpId,
        },
        user: {
          id: userId,
          name: userName,
          displayName: userName,
        },
        // ES256 (P-256/ECDSA with SHA-256) — the algorithm used by DRC-111.
        pubKeyCredParams: [{ type: 'public-key', alg: -7 }],
        authenticatorSelection: {
          // Require the key to live on the device (not a roaming authenticator).
          authenticatorAttachment: 'platform',
          // Require user verification (biometrics / PIN).
          userVerification: 'required',
          // Store the credential on the authenticator for future sign-ins.
          residentKey: 'required',
        },
        // Challenge is random; we don't verify it here — the on-chain
        // clientDataJSON verification handles that during actual signing.
        challenge: crypto.getRandomValues(new Uint8Array(32)),
        attestation: 'none',
        timeout: 60_000,
      },
    });

    if (!credential || credential.type !== 'public-key') {
      throw new Error('PasskeyWallet: credential creation failed or returned unexpected type');
    }

    const pkCredential = credential as PublicKeyCredential;
    const response = pkCredential.response as AuthenticatorAttestationResponse;

    const credentialId = new Uint8Array(pkCredential.rawId);

    // Extract the public key from the attestation object.
    // `getPublicKey()` returns a DER SubjectPublicKeyInfo; we prefer
    // `getPublicKeyBytes()` (where available) for the raw COSE key, but
    // fall back to parsing the COSE key from the authenticator data.
    let publicKey: Uint8Array;

    if (typeof response.getPublicKey === 'function') {
      const spki = response.getPublicKey();
      if (!spki) throw new Error('PasskeyWallet: getPublicKey() returned null');
      // SPKI for P-256: 26-byte header + 65-byte uncompressed point (total 91 bytes).
      // The uncompressed point starts at byte 27 (0-indexed 26).
      const spkiBytes = new Uint8Array(spki);
      if (spkiBytes.length < 27) {
        throw new Error('PasskeyWallet: unexpected SPKI length for P-256 key');
      }
      publicKey = spkiBytes.slice(26); // 65-byte uncompressed point
    } else {
      // Fallback: parse the COSE_Key from the attestedCredentialData in
      // the authenticatorData (authData) field.
      // authData layout (§6.1 of WebAuthn spec):
      //   [0..31]   rpIdHash (32 bytes)
      //   [32]      flags (1 byte)
      //   [33..36]  signCount (4 bytes, big-endian)
      //   [37..52]  AAGUID (16 bytes)
      //   [53..54]  credentialIdLength (2 bytes, big-endian)
      //   [55..]    credentialId (credentialIdLength bytes)
      //   [55+len..] COSE_Key (remaining bytes)
      const authData = new Uint8Array(response.authenticatorData);
      const credIdLen = (authData[53] << 8) | authData[54];
      const coseKeyOffset = 55 + credIdLen;
      const coseKey = authData.slice(coseKeyOffset).buffer;
      publicKey = coseKeyToUncompressedPoint(coseKey);
    }

    return { credentialId, publicKey };
  }

  /**
   * Sign a challenge using an existing passkey credential.
   *
   * Calls `navigator.credentials.get()`, prompting the user to authenticate.
   * The returned `authenticatorData`, `clientDataJSON`, `signature`, and
   * `counter` should be passed directly to the DRC-111 contract's
   * `execute_with_passkey` method.
   *
   * @param credentialId - The credential ID returned from `register()`.
   * @param challenge    - 32-byte challenge (e.g. a transaction hash or
   *                       a hash of the action to be authorised).
   * @returns WebAuthn assertion components ready for on-chain verification.
   */
  static async sign(
    credentialId: Uint8Array,
    challenge: Uint8Array,
  ): Promise<PasskeyAssertion> {
    if (typeof navigator === 'undefined' || !navigator.credentials) {
      throw new Error(
        'PasskeyWallet: navigator.credentials is not available. ' +
          'This API requires a browser environment. ' +
          'For React Native, use react-native-passkeys instead.',
      );
    }

    const assertion = await navigator.credentials.get({
      publicKey: {
        rpId: undefined, // defaults to current origin's effective domain
        allowCredentials: [
          {
            type: 'public-key',
            id: credentialId,
          },
        ],
        userVerification: 'required',
        challenge,
        timeout: 60_000,
      },
    });

    if (!assertion || assertion.type !== 'public-key') {
      throw new Error('PasskeyWallet: assertion failed or returned unexpected type');
    }

    const pkAssertion = assertion as PublicKeyCredential;
    const response = pkAssertion.response as AuthenticatorAssertionResponse;

    const authenticatorData = new Uint8Array(response.authenticatorData);
    const clientDataJSON = new Uint8Array(response.clientDataJSON);
    const signature = new Uint8Array(response.signature);

    // Extract the sign counter from authenticatorData bytes [33..36] (big-endian).
    const view = new DataView(response.authenticatorData);
    const counter = view.getUint32(33, false /* big-endian */);

    return { authenticatorData, clientDataJSON, signature, counter };
  }
}
