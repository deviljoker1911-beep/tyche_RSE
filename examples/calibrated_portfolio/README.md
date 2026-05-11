# Calibrated portfolio example

Empirically-calibrated counterpart of `examples/synthetic_portfolio/`. The
loan distribution here is sampled from the public-data priors produced by
the calibration pipeline (`data/scripts/calibrate/`).

## Regenerate

```sh
python3 -m data.scripts.calibrate
python3 examples/calibrated_portfolio/generate.py
```

The first command rebuilds `data/calibrated/*.json` from the raw datasets;
the second samples a 50-loan book + 5 scenarios that match those priors.

## Files

- `portfolio.json` — 50 loans with leverage, coupon, and sector mix drawn
  from EU empirical priors.
- `scenarios.json` — 5 named macro scenarios anchored on real episodes
  (baseline_2026, lehman_2008, eu_sovereign_2011, covid_2020,
  inflation_shock_2022).

## Run the simulator

```sh
cargo run -p tyche-cli -- simulate \
  --portfolio examples/calibrated_portfolio/portfolio.json \
  --scenarios-file examples/calibrated_portfolio/scenarios.json \
  --scenario covid_2020 --paths 20000
```

## Why both portfolios exist

- `examples/synthetic_portfolio/` — the hand-rolled spike book. Uniform
  distributions, hand-coded scenarios. Keeps the repo runnable without the
  calibration pipeline.
- `examples/calibrated_portfolio/` — empirical priors. Use this for demos,
  whitepaper figures, and design-partner conversations.
