# tyche-sim

The Monte Carlo simulation engine.

Drives a structural-credit (Merton-style) default model with a covenant-trigger
secondary channel and a seniority-aware recovery model. Correlation is induced
through a market + sector + idiosyncratic factor decomposition.

The core API is a single function:

```rust
let metrics = tyche_sim::simulate(&portfolio, &scenario, config);
```

Reproducibility: `SimConfig::seed` fully determines the output. The same
seed + same canonical-JSON inputs produce byte-identical `RiskMetrics`. This
property is the foundation of the attestation layer.

## Feature flags

- `qmc` — Sobol-sequence quasi-Monte Carlo (Phase 2 wiring; emits a `tracing`
  warning if enabled today).
- `is` — Importance sampling for tail estimation (Phase 2 wiring).
- `gpu` — CUDA acceleration (Phase 2 wiring).

The default build is CPU-only and chunked.
