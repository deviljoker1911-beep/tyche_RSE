"use client";

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { loadPortfolio, loadScenarios, fmtMoney } from "@/lib/sample-data";
import { runSimulation } from "@/lib/sim-client";
import type { RiskMetrics } from "@/lib/portfolio-schema";
import { Stat } from "@/components/stat";
import { LossHistogram } from "@/components/loss-histogram";

export default function ScenariosPage() {
  const portfolioQ = useQuery({ queryKey: ["portfolio"], queryFn: loadPortfolio });
  const scenariosQ = useQuery({ queryKey: ["scenarios"], queryFn: loadScenarios });

  const [scenarioId, setScenarioId] = useState<string>("baseline_2026");
  const [paths, setPaths] = useState<number>(10_000);
  const [metrics, setMetrics] = useState<RiskMetrics | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (portfolioQ.isLoading || scenariosQ.isLoading) {
    return <p className="text-sm text-navy-200">Loading…</p>;
  }
  const portfolio = portfolioQ.data;
  const scenarios = scenariosQ.data;
  if (!portfolio || !scenarios) {
    return <p className="text-sm text-ochre">Could not load portfolio or scenarios.</p>;
  }
  const scenario = scenarios.find((s) => s.scenario_id === scenarioId) ?? scenarios[0];
  if (!scenario) return null;

  async function run() {
    setRunning(true);
    setError(null);
    try {
      const m = await runSimulation(portfolio!, scenario!, { nPaths: paths });
      setMetrics(m);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setMetrics(null);
    } finally {
      setRunning(false);
    }
  }

  return (
    <section className="space-y-10">
      <header className="space-y-2">
        <h1 className="font-serif text-3xl tracking-tight">Scenarios</h1>
        <p className="max-w-2xl text-sm text-navy-200/80">
          Run a Monte Carlo simulation entirely in your browser. The Rust core is compiled to
          WebAssembly; nothing about your portfolio leaves this tab.
        </p>
      </header>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        <div>
          <label className="block text-xs uppercase tracking-widest text-navy-200">Scenario</label>
          <select
            value={scenarioId}
            onChange={(e) => setScenarioId(e.target.value)}
            className="mt-1 w-full rounded-md border border-navy-700/60 bg-navy-700/30 px-3 py-2 text-sm"
          >
            {scenarios.map((s) => (
              <option key={s.scenario_id} value={s.scenario_id}>
                {s.name}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="block text-xs uppercase tracking-widest text-navy-200">Paths</label>
          <input
            type="number"
            value={paths}
            min={1000}
            max={200_000}
            step={1000}
            onChange={(e) => setPaths(Number(e.target.value))}
            className="mt-1 w-full rounded-md border border-navy-700/60 bg-navy-700/30 px-3 py-2 text-sm"
          />
        </div>
        <div className="flex items-end">
          <button
            onClick={run}
            disabled={running}
            data-testid="run-simulation"
            className="w-full rounded-md bg-ochre px-4 py-2 text-sm font-medium text-canvas hover:bg-ochre-600 disabled:opacity-50"
          >
            {running ? "Running…" : "Run simulation"}
          </button>
        </div>
      </div>

      {error && <p className="text-sm text-ochre">Error: {error}</p>}

      {metrics && (
        <>
          <div className="grid grid-cols-1 gap-6 sm:grid-cols-4" data-testid="metrics">
            <Stat label="Expected loss" value={fmtMoney(metrics.expected_loss)} />
            <Stat label="VaR 95%" value={fmtMoney(metrics.var_95)} />
            <Stat label="VaR 99%" value={fmtMoney(metrics.var_99)} />
            <Stat label="ES 97.5%" value={fmtMoney(metrics.expected_shortfall_975)} />
          </div>
          <LossHistogram metrics={metrics} />
        </>
      )}
    </section>
  );
}
