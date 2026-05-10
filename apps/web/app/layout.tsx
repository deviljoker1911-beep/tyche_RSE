import type { Metadata } from "next";
import "./globals.css";
import { Providers } from "@/components/providers";
import { SiteHeader } from "@/components/site-header";

export const metadata: Metadata = {
  title: "Tyche — federated risk simulation",
  description:
    "A federated, cryptographically-attested risk simulation system for European private credit markets.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className="min-h-dvh antialiased">
        <Providers>
          <SiteHeader />
          <main className="mx-auto max-w-6xl px-6 py-10">{children}</main>
          <footer className="mx-auto max-w-6xl px-6 py-12 text-sm text-navy-200/60">
            <span className="font-mono">tyche</span> — open-source reference implementation,
            Apache-2.0
          </footer>
        </Providers>
      </body>
    </html>
  );
}
