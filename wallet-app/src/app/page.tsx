'use client';

import { useRouter } from 'next/navigation';
import { useAuth } from '@/components/AuthProvider';
import { signInWithGoogle } from '@/lib/firebase';
import { useEffect, useState } from 'react';

export default function LandingPage() {
  const { user, loading } = useAuth();
  const router = useRouter();
  const [error, setError] = useState('');
  const [signingIn, setSigningIn] = useState(false);

  // When user becomes authenticated, go to dashboard
  useEffect(() => {
    if (!loading && user) {
      router.replace('/dashboard');
    }
  }, [user, loading, router]);

  async function handleGoogleSignIn() {
    setError('');
    setSigningIn(true);
    try {
      await signInWithGoogle();
      // Don't navigate here — the useEffect above handles it
      // when onAuthStateChanged fires with the user
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : 'Sign in failed';
      setError(message);
    } finally {
      setSigningIn(false);
    }
  }

  // Show loading while checking auth state
  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="w-8 h-8 border-2 border-emerald-400 border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  // If already logged in, show loading (useEffect will redirect)
  if (user) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="w-8 h-8 border-2 border-emerald-400 border-t-transparent rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <div className="min-h-screen flex flex-col items-center justify-center px-4">
      <div className="w-full max-w-sm flex flex-col items-center gap-8">
        <div className="flex flex-col items-center gap-3">
          <div className="w-16 h-16 rounded-2xl bg-emerald-600 flex items-center justify-center text-white font-bold text-3xl shadow-lg shadow-emerald-600/20">
            D
          </div>
          <h1 className="text-4xl font-bold tracking-tight text-white">Dina</h1>
          <p className="text-slate-400 text-center text-lg">
            Your money earns. Zero fees. 100ms.
          </p>
        </div>

        <div className="w-full rounded-2xl bg-slate-900 border border-slate-800 p-6 text-center">
          <p className="text-xs text-slate-500 uppercase tracking-wider mb-2">Earning right now</p>
          <p className="text-3xl font-bold text-emerald-400 tabular-nums">4.50% APY</p>
          <p className="text-sm text-slate-400 mt-1">On every dollar, every second</p>
        </div>

        {error && (
          <div className="w-full p-3 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 text-sm text-center">
            {error}
          </div>
        )}

        <div className="w-full flex flex-col gap-3">
          <button
            onClick={handleGoogleSignIn}
            disabled={signingIn}
            className="w-full py-3 px-4 rounded-xl bg-white text-slate-950 font-semibold text-center flex items-center justify-center gap-3 hover:bg-slate-100 transition-colors disabled:opacity-50"
          >
            {signingIn ? (
              <div className="w-5 h-5 border-2 border-slate-400 border-t-transparent rounded-full animate-spin" />
            ) : (
              <svg className="w-5 h-5" viewBox="0 0 24 24">
                <path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 01-2.2 3.32v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.1z"/>
                <path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"/>
                <path fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"/>
                <path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"/>
              </svg>
            )}
            {signingIn ? 'Signing in...' : 'Sign in with Google'}
          </button>

          <button
            disabled
            className="w-full py-3 px-4 rounded-xl bg-slate-900 border border-slate-700 text-slate-500 font-semibold text-center flex items-center justify-center gap-3 cursor-not-allowed"
          >
            <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
              <path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.8-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z"/>
            </svg>
            Apple (coming soon)
          </button>

          <button
            disabled
            className="w-full py-3 px-4 rounded-xl bg-slate-900 border border-slate-700 text-slate-500 font-semibold text-center flex items-center justify-center gap-3 cursor-not-allowed"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M21.75 6.75v10.5a2.25 2.25 0 01-2.25 2.25h-15a2.25 2.25 0 01-2.25-2.25V6.75m19.5 0A2.25 2.25 0 0019.5 4.5h-15a2.25 2.25 0 00-2.25 2.25m19.5 0v.243a2.25 2.25 0 01-1.07 1.916l-7.5 4.615a2.25 2.25 0 01-2.36 0L3.32 8.91a2.25 2.25 0 01-1.07-1.916V6.75"/>
            </svg>
            Email (coming soon)
          </button>
        </div>

        <p className="text-xs text-slate-600 text-center">
          By continuing, you agree to Dina&apos;s Terms of Service and Privacy Policy.
        </p>
      </div>
    </div>
  );
}
