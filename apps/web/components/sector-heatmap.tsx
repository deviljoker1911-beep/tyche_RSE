"use client";

import { useMemo } from "react";
import { scaleLinear } from "d3-scale";
import type { Portfolio } from "@/lib/portfolio-schema";
import { fmtMoney } from "@/lib/sample-data";

const SECTORS = [
  "technology",
  "healthcare",
  "industrials",
  "consumer",
  "financials",
  "energy",
  "real_estate",
  "materials",
  "telecom",
  "utilities",
  "other",
] as const;

const GEOS = ["uk", "eu_core", "eu_periphery", "nordics", "switzerland", "em_europe", "other"] as const;

type Cell = { sector: string; geo: string; value: number };

export function SectorHeatmap({ portfolio }: { portfolio: Portfolio }) {
  const cells = useMemo<Cell[]>(() => {
    const map = new Map<string, number>();
    for (const loan of portfolio.loans) {
      const k = `${loan.sector}|${loan.geography}`;
      map.set(k, (map.get(k) ?? 0) + loan.principal);
    }
    const out: Cell[] = [];
    for (const s of SECTORS) {
      for (const g of GEOS) {
        const v = map.get(`${s}|${g}`) ?? 0;
        out.push({ sector: s, geo: g, value: v });
      }
    }
    return out;
  }, [portfolio]);

  const max = Math.max(...cells.map((c) => c.value), 1);
  const colour = scaleLinear<string>().domain([0, max]).range(["#0f1b2d", "#b15a2b"]);

  return (
    <div className="overflow-x-auto rounded-md border border-navy-700/60">
      <table className="min-w-full text-sm">
        <thead>
          <tr className="text-left text-xs uppercase tracking-widest text-navy-200/80">
            <th className="px-3 py-2">Sector</th>
            {GEOS.map((g) => (
              <th key={g} className="px-3 py-2 text-right">
                {g.replace("_", " ")}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {SECTORS.map((s) => (
            <tr key={s} className="border-t border-navy-700/40">
              <td className="px-3 py-2 font-medium capitalize text-navy-200">
                {s.replace("_", " ")}
              </td>
              {GEOS.map((g) => {
                const cell = cells.find((c) => c.sector === s && c.geo === g);
                const v = cell?.value ?? 0;
                return (
                  <td
                    key={g}
                    className="px-3 py-2 text-right font-mono tabular-nums text-canvas"
                    style={{ backgroundColor: v > 0 ? colour(v) : "transparent" }}
                    title={`${s} / ${g}: ${fmtMoney(v)}`}
                  >
                    {v > 0 ? fmtMoney(v) : "—"}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
