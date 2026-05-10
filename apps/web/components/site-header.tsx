import Link from "next/link";

const NAV = [
  { href: "/", label: "Portfolio" },
  { href: "/scenarios", label: "Scenarios" },
  { href: "/attestations", label: "Attestations" },
] as const;

export function SiteHeader() {
  return (
    <header className="border-b border-navy-700/60 bg-navy/80 backdrop-blur">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
        <Link href="/" className="flex items-baseline gap-3">
          <span className="font-serif text-xl tracking-tight text-canvas">Tyche</span>
          <span className="font-mono text-[11px] uppercase tracking-widest text-ochre">
            spike v0.1
          </span>
        </Link>
        <nav className="flex items-center gap-6 text-sm">
          {NAV.map(({ href, label }) => (
            <Link
              key={href}
              href={href}
              className="text-navy-200 transition-colors hover:text-canvas"
            >
              {label}
            </Link>
          ))}
        </nav>
      </div>
    </header>
  );
}
