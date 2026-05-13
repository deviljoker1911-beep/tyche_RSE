//! Monte Carlo credit simulation core.
//!
//! See the crate-level README for the conceptual model. This module exposes
//! [`simulate`] and the configuration knobs that govern it.
//!
//! # Determinism
//!
//! `simulate` is deterministic in `(portfolio, scenario, config)`. Internally
//! it uses [`rand_chacha::ChaCha20Rng`] seeded from `config.seed`. We **do
//! not** parallelise across paths in the spike: the reference implementation
//! must be byte-reproducible cross-platform. A future Phase 2 change will
//! introduce per-chunk RNG streams keyed off `(seed, chunk_index)` so that
//! parallel execution preserves the same property.

#![forbid(unsafe_code)]

mod factors;
mod recovery;
mod statistics;

pub use crate::factors::FactorModel;
pub use crate::recovery::RecoveryModel;
pub use crate::statistics::{LossPath, summarise};

use std::collections::BTreeMap;

use rand::SeedableRng;
use rand::distributions::{Distribution, Standard};
use rand_chacha::ChaCha20Rng;
use tyche_types::{Loan, MacroScenario, Portfolio, RiskMetrics, Sector, SectorContribution};

/// Errors that can be raised by the simulator.
#[derive(Debug, thiserror::Error)]
pub enum SimError {
    /// The portfolio failed validation.
    #[error("invalid portfolio: {0}")]
    InvalidPortfolio(String),

    /// The configuration was inconsistent.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Knobs controlling a simulation run.
///
/// `n_paths` is the total number of Monte Carlo paths. `chunk_size` controls
/// memory: paths are processed in chunks to avoid materialising
/// `n_paths × n_loans` matrices. `seed` makes the run deterministic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimConfig {
    /// Total Monte Carlo paths.
    pub n_paths: u64,
    /// Sector-factor correlation (0.0..1.0). Higher values induce more
    /// joint-default behaviour within a sector.
    pub sector_correlation: f64,
    /// Market-factor correlation (0.0..1.0). Drives systemic comovement.
    pub market_correlation: f64,
    /// Reproducibility seed for the underlying ChaCha20 RNG.
    pub seed: u64,
    /// Path-chunk size for memory-bounded simulation.
    pub chunk_size: usize,
    /// Recovery model. `RecoveryModel::SeniorityFloor` is the default.
    pub recovery_model: RecoveryModel,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            n_paths: 20_000,
            sector_correlation: 0.30,
            market_correlation: 0.20,
            seed: 0xA17EC4E_5EED_u64,
            chunk_size: 2_000,
            recovery_model: RecoveryModel::SeniorityFloor,
        }
    }
}

/// Run a Monte Carlo simulation and return aggregate risk metrics.
pub fn simulate(
    portfolio: &Portfolio,
    scenario: &MacroScenario,
    config: SimConfig,
) -> Result<RiskMetrics, SimError> {
    portfolio
        .validate()
        .map_err(|e| SimError::InvalidPortfolio(e.to_string()))?;
    if config.n_paths == 0 {
        return Err(SimError::InvalidConfig("n_paths must be > 0".into()));
    }
    if config.chunk_size == 0 {
        return Err(SimError::InvalidConfig("chunk_size must be > 0".into()));
    }
    if !(0.0..=1.0).contains(&config.sector_correlation) {
        return Err(SimError::InvalidConfig(
            "sector_correlation must be in [0, 1]".into(),
        ));
    }
    if !(0.0..=1.0).contains(&config.market_correlation) {
        return Err(SimError::InvalidConfig(
            "market_correlation must be in [0, 1]".into(),
        ));
    }

    let model = FactorModel::new(config.market_correlation, config.sector_correlation);
    let pds = derive_pd_curve(portfolio, scenario);
    let euls = exposure_under_loss(portfolio, scenario, config.recovery_model);

    let mut rng = ChaCha20Rng::seed_from_u64(config.seed);

    // Allocate path losses contiguously to make percentile extraction fast.
    let mut path_losses = Vec::with_capacity(config.n_paths as usize);
    let mut sector_path_losses: BTreeMap<Sector, f64> = BTreeMap::new();

    let mut paths_done: u64 = 0;
    while paths_done < config.n_paths {
        let chunk = std::cmp::min(config.chunk_size as u64, config.n_paths - paths_done) as usize;
        for _ in 0..chunk {
            let mut total_loss = 0.0_f64;
            // Draw one market factor for the path.
            let z_market: f64 = sample_normal(&mut rng);
            // Sector factors are conditionally independent given the market factor.
            let mut sector_z: BTreeMap<Sector, f64> = BTreeMap::new();
            for sector in iter_sectors_in(portfolio) {
                let z = sample_normal(&mut rng);
                sector_z.insert(sector, z);
            }
            for (loan, sector) in portfolio.loans.iter().zip(sector_iter(portfolio)) {
                let z_idio = sample_normal(&mut rng);
                let z_sec = sector_z.get(&sector).copied().unwrap_or(0.0);
                let z = model.combine(z_market, z_sec, z_idio);
                let pd = pds[&loan.loan_id];
                let default_threshold = inverse_normal_cdf(pd);
                let defaulted =
                    z < default_threshold || covenant_breach_default(loan, scenario, &mut rng);
                if defaulted {
                    let loss = euls[&loan.loan_id];
                    total_loss += loss;
                    *sector_path_losses.entry(sector).or_insert(0.0) += loss;
                }
            }
            path_losses.push(total_loss);
        }
        paths_done += chunk as u64;
    }

    let summary = summarise(&path_losses);
    let total_el = summary.expected_loss;
    let mut sector_contributions: BTreeMap<Sector, SectorContribution> = BTreeMap::new();
    for (sector, total) in &sector_path_losses {
        let mean = total / config.n_paths as f64;
        let share = if total_el > 0.0 { mean / total_el } else { 0.0 };
        sector_contributions.insert(
            *sector,
            SectorContribution {
                expected_loss: mean,
                share,
            },
        );
    }

    Ok(RiskMetrics {
        expected_loss: summary.expected_loss,
        var_95: summary.var_95,
        var_99: summary.var_99,
        expected_shortfall_975: summary.es_975,
        sector_contributions,
        n_paths: config.n_paths,
    })
}

fn iter_sectors_in(portfolio: &Portfolio) -> Vec<Sector> {
    let mut s: Vec<Sector> = portfolio.loans.iter().map(|l| l.sector).collect();
    s.sort_unstable();
    s.dedup();
    s
}

fn sector_iter(portfolio: &Portfolio) -> impl Iterator<Item = Sector> + '_ {
    portfolio.loans.iter().map(|l| l.sector)
}

fn sample_normal(rng: &mut ChaCha20Rng) -> f64 {
    // Box–Muller from two uniforms. We avoid the `rand_distr` dependency to
    // keep the determinism contract narrow: only `rand_chacha` and our own
    // arithmetic, no third-party distribution implementation that might
    // change between minor versions.
    let u1: f64 = sample_unit_open(rng);
    let u2: f64 = sample_unit_open(rng);
    let r = (-2.0 * u1.ln()).sqrt();
    let theta = 2.0 * std::f64::consts::PI * u2;
    r * theta.cos()
}

fn sample_unit_open(rng: &mut ChaCha20Rng) -> f64 {
    // Sample from (0, 1) — strictly open. Required for Box–Muller (ln 0 = -inf).
    loop {
        let x: f64 = Standard.sample(rng);
        if x > 0.0 && x < 1.0 {
            return x;
        }
    }
}

/// Convert a leverage figure plus a scenario into a 1-year PD via a
/// Merton-style mapping.
///
/// We model the asset value `V` as log-normal with vol `sigma_a` and drift
/// adjusted by the macro shock. Default occurs if `V_T < D` where `D` is
/// face debt. The PD is `Phi((ln D - ln V0 + ...) / (sigma_a sqrt T))`.
///
/// For the spike we collapse to a closed-form approximation parameterised on
/// `leverage = D/V0`. Phase 2 will replace this with a proper structural model.
fn derive_pd_curve(portfolio: &Portfolio, scenario: &MacroScenario) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    for loan in &portfolio.loans {
        let shock = scenario.effective_shock(loan.sector);
        // Adjust effective leverage by the EBITDA shock (more shock => higher
        // effective leverage; bounded to avoid pathological values).
        let lev_eff = loan.leverage * (1.0 - shock.ebitda_shock).max(0.05);
        // Distance to default in units of asset volatility. Anchor calibrated
        // against World Bank EU NPL medians (~1.85%) divided by a 2.5y EU
        // mid-market workout horizon, yielding a ~0.74% target median 1y PD.
        // See data/calibrated/pd_anchor.json (generated by
        // `python3 -m data.scripts.calibrate`).
        // The spike originally used 1.5, which produced PDs capped at 50%
        // across realistic books — that bug was caught by the calibration
        // pipeline.
        const PD_ANCHOR: f64 = 7.33;
        let dd = (lev_eff.ln() / loan.asset_volatility) - PD_ANCHOR;
        let pd = std_normal_cdf(dd);
        // Floor and cap to avoid numerical zeros / ones that would break the
        // inverse CDF call later.
        let pd = pd.clamp(1.0e-6, 0.5);
        out.insert(loan.loan_id.clone(), pd);
    }
    out
}

/// Loss-given-default × exposure for each loan under the scenario's recovery.
fn exposure_under_loss(
    portfolio: &Portfolio,
    scenario: &MacroScenario,
    recovery: RecoveryModel,
) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    for loan in &portfolio.loans {
        let shock = scenario.effective_shock(loan.sector);
        let recovery_rate = recovery.rate(loan, shock);
        let lgd = (1.0 - recovery_rate).clamp(0.0, 1.0);
        out.insert(loan.loan_id.clone(), loan.principal * lgd);
    }
    out
}

/// Secondary default channel: a covenant breach (cushion eroded past zero
/// under the scenario) triggers a bernoulli draw of curing vs accelerating.
fn covenant_breach_default(loan: &Loan, scenario: &MacroScenario, rng: &mut ChaCha20Rng) -> bool {
    let shock = scenario.effective_shock(loan.sector);
    if loan.covenants.is_empty() {
        return false;
    }
    // Treat cushion as fractional headroom; the EBITDA shock erodes it
    // approximately 1:1 (worst-cushion covenant dominates).
    let worst_cushion: f64 = loan
        .covenants
        .iter()
        .map(|c| c.cushion)
        .fold(f64::INFINITY, f64::min);
    let eroded = worst_cushion + shock.ebitda_shock;
    if eroded >= 0.0 {
        return false;
    }
    // Logistic mapping of the breach magnitude to an acceleration probability.
    let breach_magnitude = -eroded;
    let p_accelerate = 1.0 / (1.0 + (-3.0 * breach_magnitude + 1.5).exp());
    let u: f64 = sample_unit_open(rng);
    u < p_accelerate
}

/// Standard-normal CDF via the Abramowitz & Stegun 26.2.17 approximation.
fn std_normal_cdf(x: f64) -> f64 {
    // |error| < 7.5e-8 over the entire real line; sufficient for Monte Carlo.
    let p = 0.231_641_9;
    let b1 = 0.319_381_530;
    let b2 = -0.356_563_782;
    let b3 = 1.781_477_937;
    let b4 = -1.821_255_978;
    let b5 = 1.330_274_429;
    let abs_x = x.abs();
    let t = 1.0 / (1.0 + p * abs_x);
    let pdf = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let cdf =
        1.0 - pdf * (b1 * t + b2 * t.powi(2) + b3 * t.powi(3) + b4 * t.powi(4) + b5 * t.powi(5));
    if x >= 0.0 { cdf } else { 1.0 - cdf }
}

/// Inverse standard-normal CDF via the Beasley-Springer / Moro algorithm.
fn inverse_normal_cdf(p: f64) -> f64 {
    // Ref: Moro (1995). Coefficients lifted directly; correct over (0, 1).
    let a = [
        2.506_628_277_459_24,
        -30.664_798_066_147_2,
        138.357_751_867_269,
        -275.928_510_446_969,
        220.946_098_424_521,
    ];
    let b = [
        -54.476_098_798_224,
        161.585_836_858_041,
        -155.698_979_859_887,
        66.801_311_887_719_5,
        -13.280_681_191_127,
    ];
    let c = [
        -7.784_894_002_430_293_e-3,
        -0.322_396_458_041_136,
        -2.400_758_277_161_84,
        -2.549_732_539_343_73,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    let d = [
        7.784_695_709_041_462_e-3,
        0.322_467_159_528_795,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    let plow = 0.02425;
    let phigh = 1.0 - plow;
    if p < plow {
        let q = (-2.0 * p.ln()).sqrt();
        (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    } else if p <= phigh {
        let q = p - 0.5;
        let r = q * q;
        ((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * q
            / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
            / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tyche_types::{Covenant, CovenantDirection, Geography, Loan, Seniority};

    fn loan(id: &str, lev: f64, sector: Sector, principal: f64) -> Loan {
        Loan {
            loan_id: id.into(),
            issuer: format!("I-{id}"),
            sector,
            geography: Geography::EuCore,
            seniority: Seniority::SeniorSecured,
            principal,
            coupon: 0.075,
            maturity_years: 5.0,
            leverage: lev,
            asset_volatility: 0.30,
            covenants: vec![Covenant {
                name: "net_leverage".into(),
                threshold: 5.5,
                direction: CovenantDirection::LeqThreshold,
                cushion: 0.20,
            }],
            collateral_coverage: 1.4,
        }
    }

    fn portfolio() -> Portfolio {
        Portfolio {
            firm_id: "F-T".into(),
            as_of: "2026-03-31".into(),
            loans: vec![
                loan("L1", 4.0, Sector::Healthcare, 10.0),
                loan("L2", 5.5, Sector::Technology, 15.0),
                loan("L3", 3.5, Sector::Industrials, 20.0),
                loan("L4", 6.0, Sector::Energy, 25.0),
            ],
        }
    }

    fn scenario(name: &str, gdp: f64) -> MacroScenario {
        MacroScenario {
            scenario_id: name.into(),
            name: name.into(),
            gdp_shock: gdp,
            rate_shock_bps: 100.0,
            sector_shocks: BTreeMap::new(),
        }
    }

    #[test]
    fn deterministic_under_seed() {
        let p = portfolio();
        let s = scenario("baseline", -0.01);
        let cfg = SimConfig {
            n_paths: 1_500,
            chunk_size: 500,
            ..SimConfig::default()
        };
        let a = simulate(&p, &s, cfg).unwrap();
        let b = simulate(&p, &s, cfg).unwrap();
        assert_eq!(a, b, "same seed must produce identical metrics");
    }

    #[test]
    fn different_seeds_diverge() {
        let p = portfolio();
        let s = scenario("baseline", -0.01);
        let mut a_cfg = SimConfig::default();
        a_cfg.n_paths = 2_000;
        a_cfg.chunk_size = 500;
        a_cfg.seed = 1;
        let mut b_cfg = a_cfg;
        b_cfg.seed = 2;
        let a = simulate(&p, &s, a_cfg).unwrap();
        let b = simulate(&p, &s, b_cfg).unwrap();
        assert_ne!(a.expected_loss, b.expected_loss);
    }

    #[test]
    fn rejects_zero_paths() {
        let p = portfolio();
        let s = scenario("baseline", 0.0);
        let cfg = SimConfig {
            n_paths: 0,
            ..SimConfig::default()
        };
        assert!(simulate(&p, &s, cfg).is_err());
    }

    #[test]
    fn extreme_downturn_increases_loss() {
        let p = portfolio();
        let mild = scenario("mild", -0.005);
        let severe = scenario("severe", -0.10);
        let cfg = SimConfig {
            n_paths: 4_000,
            chunk_size: 1_000,
            ..SimConfig::default()
        };
        let a = simulate(&p, &mild, cfg).unwrap();
        let b = simulate(&p, &severe, cfg).unwrap();
        assert!(
            b.expected_loss > a.expected_loss,
            "severe should print more loss than mild: {} vs {}",
            b.expected_loss,
            a.expected_loss
        );
    }

    #[test]
    fn sector_contributions_sum_to_total() {
        let p = portfolio();
        let s = scenario("baseline", -0.02);
        let cfg = SimConfig {
            n_paths: 2_000,
            chunk_size: 500,
            ..SimConfig::default()
        };
        let m = simulate(&p, &s, cfg).unwrap();
        let summed: f64 = m
            .sector_contributions
            .values()
            .map(|c| c.expected_loss)
            .sum();
        assert!(
            (summed - m.expected_loss).abs() < 1e-9 * m.expected_loss.max(1.0),
            "sector EL sum {} vs total {}",
            summed,
            m.expected_loss
        );
    }
}
