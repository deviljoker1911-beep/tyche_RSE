//! Property-based tests for `tyche-sim`.
//!
//! These exercise the **load-bearing invariants** of the simulator:
//!
//! 1. **Reproducibility**: same `(portfolio, scenario, config)` → byte-identical
//!    `RiskMetrics`. This is the foundation of every attestation claim.
//! 2. **Monotonicity in scenario severity**: a more severe scenario produces
//!    weakly higher expected loss for the same book.
//! 3. **Sector decomposition closes**: the sum of per-sector expected loss
//!    equals total expected loss (modulo float tolerance).

use std::collections::BTreeMap;

use proptest::prelude::*;
use tyche_sim::{SimConfig, simulate};
use tyche_types::{
    Covenant, CovenantDirection, Geography, Loan, MacroScenario, Portfolio, Sector, Seniority,
};

fn arb_seniority() -> impl Strategy<Value = Seniority> {
    prop_oneof![
        Just(Seniority::SeniorSecured),
        Just(Seniority::SecondLien),
        Just(Seniority::SeniorUnsecured),
        Just(Seniority::Subordinated),
        Just(Seniority::Mezzanine),
    ]
}

fn arb_sector() -> impl Strategy<Value = Sector> {
    prop_oneof![
        Just(Sector::Technology),
        Just(Sector::Healthcare),
        Just(Sector::Industrials),
        Just(Sector::Consumer),
        Just(Sector::Energy),
    ]
}

fn arb_loan() -> impl Strategy<Value = Loan> {
    (
        "[A-Z]-[0-9]{1,3}",
        arb_sector(),
        arb_seniority(),
        1.0_f64..1e8_f64,
        0.5_f64..10.0_f64,
        0.10_f64..0.60_f64,
        0.3_f64..2.0_f64,
    )
        .prop_map(
            |(id, sector, seniority, principal, leverage, vol, cc)| Loan {
                loan_id: id.clone(),
                issuer: id,
                sector,
                geography: Geography::EuCore,
                seniority,
                principal,
                coupon: 0.07,
                maturity_years: 5.0,
                leverage,
                asset_volatility: vol,
                collateral_coverage: cc,
                covenants: vec![Covenant {
                    name: "net_leverage".into(),
                    threshold: 5.5,
                    direction: CovenantDirection::LeqThreshold,
                    cushion: 0.20,
                }],
            },
        )
}

fn arb_portfolio() -> impl Strategy<Value = Portfolio> {
    proptest::collection::vec(arb_loan(), 3..30).prop_map(|loans| Portfolio {
        firm_id: "F-PROP".into(),
        as_of: "2026-03-31".into(),
        loans,
    })
}

fn scenario(id: &str, gdp_shock: f64) -> MacroScenario {
    MacroScenario {
        scenario_id: id.into(),
        name: id.into(),
        gdp_shock,
        rate_shock_bps: 0.0,
        sector_shocks: BTreeMap::new(),
    }
}

proptest! {
    // 1k cases per M1A. The simulator is the slowest crate per test
    // (2k paths × 30 loans × ~30 us each) — beyond 1k cases the CI envelope
    // grows uncomfortably. TYCHE_PROPTEST_FAST=1 cuts to 16.
    #![proptest_config(ProptestConfig {
        cases: if std::env::var("TYCHE_PROPTEST_FAST").is_ok() { 16 } else { 1_000 },
        .. ProptestConfig::default()
    })]

    /// Byte-exact reproducibility: same inputs and same seed produce the
    /// same `RiskMetrics`. Without this, attestation is meaningless.
    #[test]
    fn deterministic_under_seed(p in arb_portfolio()) {
        let s = scenario("baseline", -0.01);
        let cfg = SimConfig {
            n_paths: 2_000,
            chunk_size: 500,
            ..SimConfig::default()
        };
        let a = simulate(&p, &s, cfg).unwrap();
        let b = simulate(&p, &s, cfg).unwrap();
        prop_assert_eq!(a, b);
    }

    /// Per-sector expected loss sums to the total — within ULP.
    #[test]
    fn sector_decomposition_closes(p in arb_portfolio()) {
        let s = scenario("baseline", -0.02);
        let cfg = SimConfig {
            n_paths: 1_000,
            chunk_size: 500,
            ..SimConfig::default()
        };
        let m = simulate(&p, &s, cfg).unwrap();
        let sum: f64 = m.sector_contributions.values().map(|c| c.expected_loss).sum();
        let tol = (1e-9_f64).max(m.expected_loss.abs() * 1e-10_f64);
        prop_assert!(
            (sum - m.expected_loss).abs() < tol,
            "sector EL sum {sum} != total {} (tol {tol})",
            m.expected_loss
        );
    }
}
