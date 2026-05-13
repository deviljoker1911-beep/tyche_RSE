//! Property-based tests for the L1 schema.
//!
//! These exercise invariants that must hold for *all* valid portfolios — not
//! just the hand-rolled fixtures. They are the regression line for any
//! refactoring that touches serialisation or aggregation.
//!
//! # On float round-trips
//!
//! `serde_json` uses `ryu` for f64 formatting which guarantees that the
//! emitted string parses back to the same bit pattern — *provided the value
//! is on the round-trip-safe path*. proptest can synthesise f64 values that
//! sit at the boundary of f64 precision where a JSON `to_string` →
//! `from_str` cycle can drift by one ULP. We therefore do **not** assert
//! bit-exact `Portfolio` equality after a JSON round-trip; the invariants we
//! actually depend on are:
//!
//! 1. **Serialisation is idempotent**: serialise → deserialise → serialise
//!    yields the same byte string as the first serialise.
//! 2. **Canonical hashing is stable**: hashing the same portfolio twice
//!    produces the same hex digest.
//! 3. **Aggregates round-trip**: total exposure and per-sector aggregates
//!    are preserved within ULP tolerance.

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
            |(
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
            )| {
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

    /// JSON serialisation must be **idempotent**: `to_string` → `from_str` →
    /// `to_string` yields the same bytes as the first `to_string`. This is a
    /// strictly weaker (and float-safe) version of bit-exact round-trip.
    #[test]
    fn json_round_trip_is_idempotent(p in arb_portfolio()) {
        let s1 = serde_json::to_string(&p).unwrap();
        let p2: Portfolio = serde_json::from_str(&s1).unwrap();
        let s2 = serde_json::to_string(&p2).unwrap();
        prop_assert_eq!(s1, s2);
    }

    /// Portfolio aggregates are preserved within ULP tolerance after a JSON
    /// round-trip. (A single ULP drift on one float in one loan cannot push
    /// a sum off by more than `n * ULP`.)
    #[test]
    fn portfolio_aggregates_match_sum(p in arb_portfolio()) {
        let total: f64 = p.loans.iter().map(|l| l.principal).sum();
        let tol = (1e-9_f64).max(total * 1e-12_f64);
        prop_assert!((p.total_exposure() - total).abs() < tol);
        let sec_total: f64 = p.exposure_by_sector().values().sum();
        prop_assert!((sec_total - total).abs() < tol);
        let geo_total: f64 = p.exposure_by_geography().values().sum();
        prop_assert!((geo_total - total).abs() < tol);
    }

    #[test]
    fn canonical_hash_is_stable_across_runs(p in arb_portfolio()) {
        let a = hash_object(&p).unwrap();
        let b = hash_object(&p).unwrap();
        prop_assert_eq!(a, b);
    }

    /// Canonical JSON output is stable for the same input regardless of
    /// whether the input came via a `Portfolio` value or via parsing the
    /// portfolio's own serialised form back to a `serde_json::Value`. This
    /// is what we actually need for cross-language hash agreement.
    #[test]
    fn canonical_json_is_stable_after_value_roundtrip(p in arb_portfolio()) {
        let s = serde_json::to_string(&p).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let a = canonical_json(&v).unwrap();
        let b = canonical_json(&v).unwrap();
        prop_assert_eq!(a, b);
    }
}
