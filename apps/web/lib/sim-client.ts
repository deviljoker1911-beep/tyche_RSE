/**
 * Browser-side simulation client.
 *
 * Loads the wasm module emitted by `crates/tyche-wasm` and exposes a typed
 * wrapper around its exports. The module is fetched on first call and cached
 * for the lifetime of the page.
 */

import type { MacroScenario, Portfolio, RiskMetrics } from "./portfolio-schema";
import { RiskMetricsZ } from "./portfolio-schema";

interface TycheWasm {
  default: (input?: { module_or_path?: string }) => Promise<unknown>;
  simulate: (inputJson: string) => unknown;
  buildAttestation: (inputJson: string) => unknown;
  verifyAttestation: (recordJson: string) => boolean;
}

let modulePromise: Promise<TycheWasm> | null = null;

async function loadWasm(): Promise<TycheWasm> {
  if (!modulePromise) {
    // The wasm-pack output is staged into apps/web/public/wasm by scripts/dev.sh.
    modulePromise = import(/* webpackIgnore: true */ "/wasm/tyche_wasm.js").then(
      async (mod) => {
        const wasm = mod as unknown as TycheWasm;
        await wasm.default();
        return wasm;
      },
    );
  }
  return modulePromise;
}

export interface SimulateOpts {
  nPaths?: number;
  seed?: number;
}

export async function runSimulation(
  portfolio: Portfolio,
  scenario: MacroScenario,
  opts: SimulateOpts = {},
): Promise<RiskMetrics> {
  const wasm = await loadWasm();
  const result = wasm.simulate(
    JSON.stringify({
      portfolio,
      scenario,
      config: {
        n_paths: opts.nPaths ?? 20_000,
        seed: opts.seed ?? 0xa17ec4e_5eed,
      },
    }),
  );
  return RiskMetricsZ.parse(result);
}
