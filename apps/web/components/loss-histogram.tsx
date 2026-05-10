"use client";

/**
 * Loss-distribution view.
 *
 * The simulator returns aggregate metrics rather than the full path vector
 * (cheaper over the wasm boundary), so this component visualises the sector
 * decomposition as a bar chart instead of a histogram of paths. When
 * Phase 2 adds a "stream paths back" mode, this will become a true histogram.
 */

import type { RiskMetrics } from "@/lib/portfolio-schema";
import { fmtMoney } from "@/lib/sample-data";

export function LossHistogram({ metrics }: { metrics: RiskMetrics }) {
  const entries = Object.entries(metrics.sector_contributions).sort(
    (a, b) => b[1].expected_loss - a[1].expected_loss,
  );
  const max = Math.max(...entries.map(([, v]) => v.expected_loss), 1);

  return (
    <div className="space-y-2">
      <h2 className="text-sm font-medium uppercase tracking-widest text-navy-200">
        Sector contributions to expected loss
      </h2>
      <div className="space-y-1 rounded-md border border-navy-700/60 bg-navy-700/20 p-4">
        {entries.map(([sector, c]) => (
          <div key={sector} className="grid grid-cols-12 items-center gap-3">
            <span className="col-span-3 text-sm text-navy-200">
              {sector.replace("_", " ")}
            </span>
            <div className="col-span-7 h-3 overflow-hidden rounded-full bg-navy-700/60">
              <div
                className="h-full bg-ochre"
                style={{ width: `${(c.expected_loss / max) * 100}%` }}
              />
            </div>
            <span className="col-span-2 text-right font-mono text-xs tabular-nums">
              {fmtMoney(c.expected_loss)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
