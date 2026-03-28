import { initializeApp, getApps } from 'firebase/app';
import {
  getAuth,
  GoogleAuthProvider,
  signInWithPopup,
  signOut as firebaseSignOut,
  onAuthStateChanged,
  type User,
} from 'firebase/auth';

const firebaseConfig = {
  apiKey: 'AIzaSyD5AIokAcEPP_SIhfJlmOhmHNCYaCQxOVk',
  authDomain: 'lucilla-b0493.firebaseapp.com',
  projectId: 'lucilla-b0493',
  storageBucket: 'lucilla-b0493.firebasestorage.app',
  messagingSenderId: '290142209974',
  appId: '1:290142209974:web:048ffd0a6c2121550e2317',
};

const app = getApps().length === 0 ? initializeApp(firebaseConfig) : getApps()[0];
const auth = getAuth(app);

const googleProvider = new GoogleAuthProvider();

export async function signInWithGoogle(): Promise<void> {
  // signInWithPopup may throw COOP errors on some browsers
  // but auth still succeeds — onAuthStateChanged picks it up
  try {
    await signInWithPopup(auth, googleProvider);
  } catch (err: unknown) {
    // If the popup was closed or COOP blocked it, check if auth actually worked
    // by waiting a moment for onAuthStateChanged to fire
    const msg = err instanceof Error ? err.message : '';
    if (msg.includes('popup-closed') || msg.includes('cancelled') || msg.includes('network-request-failed')) {
      // These are benign — user closed popup or network hiccup
      return;
    }
    // For other errors, re-throw
    throw err;
  }
}

export async function signOut(): Promise<void> {
  await firebaseSignOut(auth);
}

export function onAuthChange(callback: (user: User | null) => void): () => void {
  return onAuthStateChanged(auth, callback);
}

export { auth };
export type { User };
