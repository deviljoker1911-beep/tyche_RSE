/**
 * Loaders for the synthetic spike portfolio. The web app ships these files in
 * `public/data/` so the demo can run without a backend.
 */

import { MacroScenarioZ, PortfolioZ, type MacroScenario, type Portfolio } from "./portfolio-schema";

export async function loadPortfolio(): Promise<Portfolio> {
  const res = await fetch("/data/portfolio.json", { cache: "force-cache" });
  if (!res.ok) throw new Error(`portfolio fetch failed: ${res.status}`);
  return PortfolioZ.parse(await res.json());
}

export async function loadScenarios(): Promise<MacroScenario[]> {
  const res = await fetch("/data/scenarios.json", { cache: "force-cache" });
  if (!res.ok) throw new Error(`scenarios fetch failed: ${res.status}`);
  const raw = (await res.json()) as unknown;
  if (!Array.isArray(raw)) throw new Error("scenarios.json must be an array");
  return raw.map((entry) => MacroScenarioZ.parse(entry));
}

export function fmtMoney(n: number): string {
  if (Math.abs(n) >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(2)}B`;
  if (Math.abs(n) >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
  if (Math.abs(n) >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return n.toFixed(0);
}
