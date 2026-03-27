"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.DinaWallet = void 0;
const ed = __importStar(require("@noble/ed25519"));
const sha512_1 = require("@noble/hashes/sha512");
const utils_1 = require("./utils");
// ed25519 v2 requires setting the sha512 hash function
ed.etc.sha512Sync = (...m) => {
    const h = sha512_1.sha512.create();
    for (const msg of m)
        h.update(msg);
    return h.digest();
};
/**
 * Ed25519 wallet for the Dina Network.
 *
 * Wraps @noble/ed25519 to provide key generation, signing, and verification.
 * The wallet derives a Dina address from the public key via SHA-256.
 */
class DinaWallet {
    constructor(privateKey, publicKey) {
        this._privateKey = privateKey;
        this.publicKey = publicKey;
        this.address = (0, utils_1.addressFromPublicKey)(publicKey);
    }
    /** Generate a new random wallet. */
    static generate() {
        const privateKey = ed.utils.randomPrivateKey();
        const publicKey = ed.getPublicKey(privateKey);
        return new DinaWallet(privateKey, publicKey);
    }
    /**
     * Restore a wallet from a private key.
     * @param key - 32-byte private key as Uint8Array or 64-char hex string.
     */
    static fromPrivateKey(key) {
        const privateKey = typeof key === 'string' ? (0, utils_1.hexToBytes)(key) : key;
        if (privateKey.length !== 32) {
            throw new Error(`Private key must be 32 bytes, got ${privateKey.length}`);
        }
        const publicKey = ed.getPublicKey(privateKey);
        return new DinaWallet(privateKey, publicKey);
    }
    /**
     * Restore a wallet from a BIP-39 mnemonic phrase.
     *
     * Derives the private key by taking SHA-256 of the mnemonic bytes.
     * A full BIP-39 + SLIP-0010 derivation path implementation would be used
     * in production; this provides a deterministic key from the phrase.
     */
    static fromMnemonic(mnemonic) {
        const trimmed = mnemonic.trim();
        const words = trimmed.split(/\s+/);
        if (words.length < 12 || words.length > 24) {
            throw new Error(`Mnemonic must be 12-24 words, got ${words.length}`);
        }
        // Derive 32-byte seed from mnemonic via SHA-256
        // In production this would use PBKDF2 per BIP-39 spec
        const { sha256 } = require('@noble/hashes/sha256');
        const seed = sha256(new TextEncoder().encode(trimmed));
        return DinaWallet.fromPrivateKey(seed);
    }
    /** Sign a message, returning a hex-encoded 64-byte Ed25519 signature. */
    sign(message) {
        const sig = ed.sign(message, this._privateKey);
        return (0, utils_1.bytesToHex)(sig);
    }
    /** Verify a signature against a message using this wallet's public key. */
    verify(message, signature) {
        const sigBytes = (0, utils_1.hexToBytes)(signature);
        return ed.verify(sigBytes, message, this.publicKey);
    }
    /** Export public information (never exposes the private key). */
    toJSON() {
        return {
            address: this.address,
            publicKey: (0, utils_1.bytesToHex)(this.publicKey),
        };
    }
    /** Export the private key as hex. Handle with care. */
    exportPrivateKey() {
        return (0, utils_1.bytesToHex)(this._privateKey);
    }
}
exports.DinaWallet = DinaWallet;
//# sourceMappingURL=wallet.js.map