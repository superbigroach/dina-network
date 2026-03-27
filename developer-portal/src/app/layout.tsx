import type { Metadata } from "next";
import { Inter } from "next/font/google";
import Link from "next/link";
import "./globals.css";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
});

export const metadata: Metadata = {
  title: "Dina Network — Developer Portal",
  description:
    "Build on the fastest blockchain ever. 100,000+ TPS, 100ms finality, USDC-native, 82 DRC smart contract standards.",
};

const NAV_LINKS = [
  { label: "Docs", href: "/docs" },
  { label: "Faucet", href: "/faucet" },
  { label: "Explorer", href: "/explorer" },
  { label: "Wallets", href: "/wallets" },
  { label: "GitHub", href: "https://github.com/superbigroach/dina-network", external: true },
];

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={inter.variable}>
      <body className="bg-slate-950 text-white antialiased min-h-screen flex flex-col">
        {/* Top Navbar */}
        <header className="sticky top-0 z-50 border-b border-slate-800/60 bg-slate-950/80 backdrop-blur-xl">
          <nav className="mx-auto flex max-w-7xl items-center justify-between px-6 py-4">
            {/* Logo */}
            <Link href="/" className="flex items-center gap-2.5 group">
              <div className="h-8 w-8 rounded-lg bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center text-sm font-bold tracking-tight shadow-lg shadow-blue-500/20 group-hover:shadow-blue-500/40 transition-shadow">
                D
              </div>
              <span className="text-lg font-semibold tracking-tight">
                Dina Network
              </span>
            </Link>

            {/* Nav Links */}
            <div className="flex items-center gap-1">
              {NAV_LINKS.map((link) => (
                <Link
                  key={link.href}
                  href={link.href}
                  {...(link.external
                    ? { target: "_blank", rel: "noopener noreferrer" }
                    : {})}
                  className="rounded-lg px-3.5 py-2 text-sm font-medium text-slate-300 transition-colors hover:bg-slate-800/60 hover:text-white"
                >
                  {link.label}
                </Link>
              ))}
            </div>
          </nav>
        </header>

        {/* Main Content */}
        <main className="flex-1">{children}</main>
      </body>
    </html>
  );
}
