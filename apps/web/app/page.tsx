"use client";

import { useQuery } from "@tanstack/react-query";
import { fmtMoney, loadPortfolio } from "@/lib/sample-data";
import { SectorHeatmap } from "@/components/sector-heatmap";
import { TopIssuersTable } from "@/components/top-issuers-table";
import { Stat } from "@/components/stat";

export default function PortfolioPage() {
  const { data: portfolio, error, isLoading } = useQuery({
    queryKey: ["portfolio"],
    queryFn: loadPortfolio,
  });

  if (isLoading) {
    return <p className="text-sm text-navy-200">Loading portfolio…</p>;
  }
  if (error || !portfolio) {
    return (
      <p className="text-sm text-ochre">
        Portfolio could not be loaded. Make sure `public/data/portfolio.json` is in place.
      </p>
    );
  }

  const total = portfolio.loans.reduce((acc, l) => acc + l.principal, 0);
  const issuers = new Set(portfolio.loans.map((l) => l.issuer)).size;

  return (
    <section className="space-y-10">
      <header className="space-y-2">
        <h1 className="font-serif text-3xl tracking-tight">Portfolio</h1>
        <p className="max-w-2xl text-sm text-navy-200/80">
          Synthetic European mid-market direct-lending book used for the spike demo. As-of{" "}
          <span className="font-mono">{portfolio.as_of}</span>, firm{" "}
          <span className="font-mono">{portfolio.firm_id}</span>.
        </p>
      </header>

      <div className="grid grid-cols-1 gap-6 sm:grid-cols-3">
        <Stat label="Total exposure" value={fmtMoney(total)} hint="aggregate principal outstanding" />
        <Stat label="Loans" value={portfolio.loans.length.toString()} hint="positions in the book" />
        <Stat label="Issuers" value={issuers.toString()} hint="unique borrowers" />
      </div>

      <div className="grid grid-cols-1 gap-8 lg:grid-cols-5">
        <div className="lg:col-span-3">
          <h2 className="mb-4 text-sm font-medium uppercase tracking-widest text-navy-200">
            Exposure by sector
          </h2>
          <SectorHeatmap portfolio={portfolio} />
        </div>
        <div className="lg:col-span-2">
          <h2 className="mb-4 text-sm font-medium uppercase tracking-widest text-navy-200">
            Top issuers
          </h2>
          <TopIssuersTable portfolio={portfolio} k={10} />
        </div>
      </div>
    </section>
  );
}
