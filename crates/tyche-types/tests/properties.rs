//! Property-based tests for the L1 schema.
//!
//! These exercise invariants that must hold for *all* valid portfolios — not
//! just the hand-rolled fixtures. They are the regression line for any
//! refactoring that touches serialisation or aggregation.

use proptest::prelude::*;
use tyche_types::{
    Covenant, CovenantDirection, Geography, Loan, Portfolio, Sector, Seniority, canonical_json,
    hash_object,
};

fn arb_seniority() -> impl Strategy<Value = Seniority> {
    prop_oneof![
        Just(Seniority::SeniorSecured),
        Just(Seniority::SecondLien),
        Just(Seniority::SeniorUnsecured),
        Just(Seniority::Subordinated),
        Just(Seniority::Mezzanine),
        Just(Seniority::Equity),
    ]
}

fn arb_sector() -> impl Strategy<Value = Sector> {
    prop_oneof![
        Just(Sector::Technology),
        Just(Sector::Healthcare),
        Just(Sector::Industrials),
        Just(Sector::Consumer),
        Just(Sector::Financials),
        Just(Sector::Energy),
        Just(Sector::RealEstate),
        Just(Sector::Materials),
        Just(Sector::Telecom),
        Just(Sector::Utilities),
        Just(Sector::Other),
    ]
}

fn arb_geography() -> impl Strategy<Value = Geography> {
    prop_oneof![
        Just(Geography::Uk),
        Just(Geography::EuCore),
        Just(Geography::EuPeriphery),
        Just(Geography::Nordics),
        Just(Geography::Switzerland),
        Just(Geography::EmEurope),
        Just(Geography::Other),
    ]
}

fn arb_loan() -> impl Strategy<Value = Loan> {
    (
        "[A-Z]-[0-9]{1,4}",
        "[A-Z]{1,3}-[0-9]{1,4}",
        arb_sector(),
        arb_geography(),
        arb_seniority(),
        1.0_f64..1e9_f64,
        0.0_f64..0.25_f64,
        0.25_f64..15.0_f64,
        0.0_f64..15.0_f64,
        0.05_f64..1.5_f64,
        0.0_f64..3.0_f64,
    )
        .prop_map(
            |(loan_id, issuer, sector, geography, seniority, principal, coupon, maturity_years, leverage, asset_volatility, collateral_coverage)| {
                Loan {
                    loan_id,
                    issuer,
                    sector,
                    geography,
                    seniority,
                    principal,
                    coupon,
                    maturity_years,
                    leverage,
                    asset_volatility,
                    collateral_coverage,
                    covenants: vec![Covenant {
                        name: "net_leverage".into(),
                        threshold: 5.5,
                        direction: CovenantDirection::LeqThreshold,
                        cushion: 0.15,
                    }],
                }
            },
        )
}

fn arb_portfolio() -> impl Strategy<Value = Portfolio> {
    proptest::collection::vec(arb_loan(), 1..50).prop_map(|loans| Portfolio {
        firm_id: "F-PROP".into(),
        as_of: "2026-03-31".into(),
        loans,
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, .. ProptestConfig::default() })]

    #[test]
    fn portfolio_round_trips_through_json(p in arb_portfolio()) {
        let s = serde_json::to_string(&p).unwrap();
        let p2: Portfolio = serde_json::from_str(&s).unwrap();
        prop_assert_eq!(p, p2);
    }

    #[test]
    fn portfolio_aggregates_match_sum(p in arb_portfolio()) {
        let total: f64 = p.loans.iter().map(|l| l.principal).sum();
        prop_assert!((p.total_exposure() - total).abs() < 1e-6 * total.max(1.0));
        let sec_total: f64 = p.exposure_by_sector().values().sum();
        prop_assert!((sec_total - total).abs() < 1e-6 * total.max(1.0));
        let geo_total: f64 = p.exposure_by_geography().values().sum();
        prop_assert!((geo_total - total).abs() < 1e-6 * total.max(1.0));
    }

    #[test]
    fn canonical_hash_is_stable_across_runs(p in arb_portfolio()) {
        let a = hash_object(&p).unwrap();
        let b = hash_object(&p).unwrap();
        prop_assert_eq!(a, b);
    }

    #[test]
    fn canonical_json_does_not_depend_on_map_ordering(p in arb_portfolio()) {
        // Re-serialise once, parse to a serde_json::Value (which preserves
        // insertion order) and then re-canonicalise: must be byte-identical.
        let s = serde_json::to_string(&p).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let a = canonical_json(&p).unwrap();
        let b = canonical_json(&v).unwrap();
        prop_assert_eq!(a, b);
    }
}
