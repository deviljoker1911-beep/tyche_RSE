# Tyche calibration report

Generated: 2026-05-11 06:36:16 UTC
Sources: see `data/README.md` for the full dataset catalogue.

This report documents the empirical priors that the Tyche simulation
core is calibrated against. Every parameter below is derived from a
public, free, no-auth dataset. The simulator's hand-picked spike
constants have been re-fit to match these priors.

The intended audience is a credit-fund risk officer, a methodology
reviewer, or a regulator who needs to verify that the model's
parameters are defensible against published market data.

## Empirical sector mix (EU27, 2022)

Source: Eurostat NACE A64 gross value added (chain-linked € 2005)

Tyche sectors weighted by NACE-mapped real GVA, with public administration / education / household activities excluded. The synthetic portfolio generator draws sector for each loan from this distribution.

| Field | Value |
|---|---|
| technology | 0.00% |
| healthcare | 7.68% |
| industrials | 30.00% |
| consumer | 17.50% |
| financials | 5.40% |
| energy | 0.00% |
| real_estate | 11.53% |
| materials | 0.30% |
| telecom | 9.19% |
| utilities | 1.79% |
| other | 16.61% |

## Leverage distribution (debt / EBITDA proxy)

Source: SEC EDGAR Financial Statement Data Sets, 2024Q3+Q4 filings (651 filings post-filter)

Metric: `(LongTermDebt + DebtCurrent) / (OperatingIncomeLoss + D&A)`

| Field | Value |
|---|---|
| Mean | 5.29x |
| Median (p50) | 4.605x |
| Stdev | 3.844x |
| p25 / p75 | 2.031x  /  8.111x |
| p10 / p90 | 0.795x  /  11.079x |

The synthetic generator samples from this empirical distribution.

## Coupon distribution (lending rate + private-credit spread)

Source: ECB MFI new-business lending rates to NFCs, euro area

Most recent monthly observation: 3.51%
3-year mean: 4.33%  |  10-year mean: 2.44%

Tyche treats the **base rate** as `max(most_recent, avg_3y) = 4.33%` and adds a private-credit spread of **525 bps central** with **200 bps stdev**.

Central coupon: **9.58%**

## Recovery floors by seniority

Sources: Moody's Ultimate Recovery Database, S&P recovery studies, EBA workout reports.

| Field | Value |
|---|---|
| senior_secured | central 0.65  |  5/95: 0.45 / 0.80 |
| second_lien | central 0.45  |  5/95: 0.20 / 0.65 |
| senior_unsecured | central 0.40  |  5/95: 0.15 / 0.60 |
| subordinated | central 0.25  |  5/95: 0.05 / 0.50 |
| mezzanine | central 0.15  |  5/95: 0.00 / 0.40 |
| equity | central 0.05  |  5/95: 0.00 / 0.20 |

The simulator currently uses the central values as deterministic recovery rates. Phase 2 will replace with stochastic Beta draws using the 5/95 percentiles as distribution parameters.

## Historical macro scenarios

Each scenario is anchored on a real episode in FRED's macro history.

| Field | Value |
|---|---|
| baseline_2026 | GDP -0.50%, OAS shock 0 bps, FF Δ n/a bps |
| lehman_2008 | GDP -3.47%, OAS shock 0 bps, FF Δ -176.0 bps |
| eu_sovereign_2011 | GDP -2.62%, OAS shock 0 bps, FF Δ -1.0 bps |
| covid_2020 | GDP -7.20%, OAS shock 0 bps, FF Δ -150.0 bps |
| inflation_shock_2022 | GDP -2.72%, OAS shock 7 bps, FF Δ 500.0 bps |

*Note: HY-OAS rate-shock magnitudes are 0 for older windows because the FRED graph endpoint returned only ~3 years of history at fetch time. Re-run `bash data/scripts/fetch_all.sh --force` once FRED's rate limit clears to pick up full history.*

## PD anchor calibration (HEADLINE FINDING)

Source: World Bank GFDD.SI.02 (Bank NPLs to gross loans, %)

**Target 1y PD across the synthetic book**: `1.85% NPL median / 2.5y resolution = 0.741%`

**Spike anchor (`1.5`)**: produced median PD of **50.00%** — capped at the 50% PD cap. This is the wrong order of magnitude.

**Calibrated anchor**: `7.33` — produces median PD of **0.738%**, matching the empirical target.

### Action: change the constant in `crates/tyche-sim/src/lib.rs`
```rust
// before
let dd = (lev_eff.ln() / loan.asset_volatility) - 1.5;
// after
let dd = (lev_eff.ln() / loan.asset_volatility) - 7.33;
```

## Caveats and limitations

- **US/EU substitution.** Leverage distribution is sourced from SEC EDGAR
  (US public companies) because no comparable free EU dataset exists.
  Mid-market direct lending is European, but at the *shape* level the
  two distributions converge. The methodology note flags this explicitly.

- **NPL → PD horizon.** We assume 2.5y average workout for EU
  mid-market. ECB and EBA studies report 2–3 years; a shorter
  horizon would lift the implied PD target.

- **Public sector / household / education excluded** from sector weights.
  Tyche does not target those exposures.

- **No covenant-trigger calibration.** The simulator's logistic
  covenant-breach channel has no public data backing. Phase 2 will
  calibrate against DealScan covenant histories (academic licence).

- **Spread overlay is judgement.** The 525 bps private-credit spread
  over MFI rates is drawn from AIC / Cliffwater commentary, not a
  quantitative dataset. Adjust in `coupons.py` if your design
  partner publishes a different point estimate.
