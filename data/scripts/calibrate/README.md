# Tyche calibration pipeline

Turns the public datasets under `data/raw/` and `data/processed/` into a set
of small, human-readable JSON artifacts under `data/calibrated/` that the
Tyche simulation core (and its synthetic-portfolio generator) consume.

The pipeline is the single bridge between *"reasonable guesses"* and
*"defensible against published market data"*. Every parameter in the
simulator now traces back to a citable public source.

## Run it

```sh
# from repo root
python3 -m data.scripts.calibrate
```

This runs every submodule in dependency order and writes
`data/calibrated/CALIBRATION_REPORT.md` along with one JSON artifact per
domain. The report is a self-contained methodology document — short
enough to send to a credit-fund risk officer; long enough to act as
§Calibration in the eventual whitepaper.

## Modules

| Module | Source | Output |
|---|---|---|
| `sectors.py` | Eurostat NACE A64 GVA | `sector_weights.json` |
| `leverage.py` | SEC EDGAR FSDS | `leverage_distribution.json` |
| `coupons.py` | ECB MFI new-business lending rates | `coupon_distribution.json` |
| `recovery.py` | Moody's URD / S&P / EBA workout reports | `recovery_floors.json` |
| `scenarios.py` | FRED macro time series | `historical_scenarios.json` |
| `pds.py` | World Bank GFDD.SI.02 + the leverage output | `pd_anchor.json` |

## Headline finding

The spike's hand-picked PD anchor of `1.5` (in
`crates/tyche-sim/src/lib.rs::derive_pd_curve`) produced PDs that hit the
50% cap on every realistic loan. The calibration pipeline re-fit the
anchor against EU NPL ratios at **`7.33`**, which produces a median 1y PD
of `~0.74%` — matching the empirical European mid-market target. The
constant has been updated in the simulator.

## Use with the simulator

```sh
# Generate a calibrated synthetic book + scenarios
python3 examples/calibrated_portfolio/generate.py

# Run the simulator against them
cargo run -p tyche-cli -- simulate \
  --portfolio examples/calibrated_portfolio/portfolio.json \
  --scenarios-file examples/calibrated_portfolio/scenarios.json \
  --scenario covid_2020 \
  --paths 20000
```

## What this pipeline is **not**

- It is **not** a substitute for vendor data (DealScan, S&P CreditPro,
  Preqin) for production-grade backtesting. The leverage proxy uses US
  public companies, the recovery floors are hand-anchored against published
  studies, and there's no covenant calibration.
- It is **not** sufficient on its own to defend a model to a Tier-1
  regulator — that requires real loan-level outcomes, which require paid
  data and a design partner.
- It **is** sufficient to publish a methodology paper, run a credible
  demo, and onboard the first design partner. That's the role it plays.
