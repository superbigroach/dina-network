import type { Metadata } from 'next';
import { AuthProvider } from '@/components/AuthProvider';
import '@/styles/globals.css';

export const metadata: Metadata = {
  title: 'Dina Wallet',
  description: 'Your money earns. Zero fees. 100ms.',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="antialiased min-h-screen bg-slate-950 text-white">
        <AuthProvider>{children}</AuthProvider>
      </body>
    </html>
  );
}
