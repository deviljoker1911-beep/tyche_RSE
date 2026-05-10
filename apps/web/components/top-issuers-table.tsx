"use client";

import { useMemo } from "react";
import type { Portfolio } from "@/lib/portfolio-schema";
import { fmtMoney } from "@/lib/sample-data";

export function TopIssuersTable({ portfolio, k }: { portfolio: Portfolio; k: number }) {
  const rows = useMemo(() => {
    const m = new Map<string, number>();
    for (const loan of portfolio.loans) {
      m.set(loan.issuer, (m.get(loan.issuer) ?? 0) + loan.principal);
    }
    return [...m.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, k)
      .map(([issuer, exposure]) => ({ issuer, exposure }));
  }, [portfolio, k]);

  const total = portfolio.loans.reduce((acc, l) => acc + l.principal, 0);

  return (
    <div className="overflow-hidden rounded-md border border-navy-700/60">
      <table className="min-w-full text-sm">
        <thead>
          <tr className="text-left text-xs uppercase tracking-widest text-navy-200/80">
            <th className="px-4 py-2">Issuer</th>
            <th className="px-4 py-2 text-right">Exposure</th>
            <th className="px-4 py-2 text-right">Share</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.issuer} className="border-t border-navy-700/40">
              <td className="px-4 py-2 font-mono text-navy-200">{r.issuer}</td>
              <td className="px-4 py-2 text-right font-mono tabular-nums">{fmtMoney(r.exposure)}</td>
              <td className="px-4 py-2 text-right font-mono tabular-nums text-ochre">
                {((r.exposure / total) * 100).toFixed(1)}%
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
