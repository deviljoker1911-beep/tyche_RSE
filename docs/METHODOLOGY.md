# Tyche methodology

This document is a reproducible specification of the Tyche credit simulation
model used in the spike phase. The forthcoming whitepaper will extend this
write-up with a full literature review, parameter calibration against
European mid-market direct-lending data, and a discussion of model risk.

## Inputs

A single simulation run consumes:

- a `Portfolio` (`tyche-types::Portfolio`) — a list of `Loan` records,
- a `MacroScenario` — macro and sector shocks,
- a `SimConfig` — number of paths, factor correlations, RNG seed, recovery
  model, chunk size.

Every numeric quantity is in the portfolio's reporting currency. The spike
does not model FX explicitly.

## Default model

Default for loan _i_ over the simulation horizon is governed by a structural
asset-value process. We use a closed-form approximation parameterised on net
leverage `λ_i` and asset volatility `σ_i`:

```
PD_i = Φ( ln(λ_i_eff) / σ_i  −  1.5 )

where  λ_i_eff = λ_i · max(0.05, 1 − ε_i_ebitda)
       ε_i_ebitda is the scenario's EBITDA shock for loan i's sector
       Φ is the standard-normal CDF
```

The constant `1.5` is a calibration anchor chosen so that a healthy 4.0x
leverage at 30% asset volatility prints a 1y PD around 1.5–2.0%, consistent
with European mid-market direct-lending priors.

PDs are clamped to `[1e-6, 0.5]` to avoid pathological values entering the
inverse-normal CDF later.

## Correlated default draws

For each path we draw:

- one market-factor innovation `z_m ~ N(0, 1)`,
- one sector-factor innovation `z_s ~ N(0, 1)` per sector present in the book,
- one idiosyncratic innovation `z_idio ~ N(0, 1)` per loan.

Each loan's combined innovation is

```
z_i = sqrt(ρ_m) · z_m + sqrt(ρ_s) · z_s_sec(i) + sqrt(1 − ρ_m − ρ_s) · z_idio_i
```

with `ρ_m, ρ_s in [0, 1]` and `ρ_m + ρ_s <= 1`. The simulator normalises to
preserve unit variance if a misconfigured caller violates the budget.

A loan defaults on a path when

```
z_i  <  Φ⁻¹(PD_i)        (primary structural channel)
   or
covenant_breach(loan, scenario)  (secondary channel; see below)
```

## Covenant-breach channel

Each `Loan` carries a list of `Covenant` records with a fractional `cushion`
field. Under a scenario, the worst cushion is eroded by the EBITDA shock:

```
cushion_eff = min(c.cushion for c in covenants) + ε_ebitda
```

If `cushion_eff < 0`, a logistic acceleration probability is computed:

```
p_accelerate = sigmoid( 3 · (−cushion_eff) − 1.5 )
```

A bernoulli draw against `p_accelerate` triggers default on the path.

## Recovery

Two models are exposed; the default is `SeniorityCollateralized`:

- **`SeniorityFloor`** — recovery equals the seniority floor, ignoring
  collateral and macro state. Useful as a sanity baseline.
- **`SeniorityCollateralized`** — recovery starts at the seniority floor,
  is uplifted (or pulled down) by `(collateral_coverage - 1) · 0.30`, and
  finally dampened by `(1 + asset_shock)` to capture mark-to-market on
  pledged collateral. Result is clamped to `[0, 0.95]`.

LGD is `1 − recovery`. Loss given default is `principal · LGD`.

## Output statistics

After `n_paths` paths are drawn we compute:

- expected loss = mean of path losses,
- VaR 95%, VaR 99% — empirical quantiles on the sorted loss vector,
- ES 97.5% — mean of the worst 2.5% tail,
- per-sector EL contributions, normalised to a share field.

## Non-goals (Phase 2)

- Multi-horizon simulation (today: 1-year only).
- FX and rate modelling beyond a parallel front-end shock.
- Quasi-Monte Carlo and importance sampling (feature flags exist, no
  implementation).
- Calibration against named real datasets — the spike runs on a synthetic
  portfolio.
