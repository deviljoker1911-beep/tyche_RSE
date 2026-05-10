//! Throughput benchmark for `tyche_sim::simulate`.
//!
//! Goal from the spike spec: a 100-loan portfolio at 20k paths in <200ms on a
//! modern laptop. Run with `cargo bench -p tyche-sim`.

use std::collections::BTreeMap;

use criterion::{Criterion, criterion_group, criterion_main};
use tyche_sim::{SimConfig, simulate};
use tyche_types::{
    Covenant, CovenantDirection, Geography, Loan, MacroScenario, Portfolio, Sector, Seniority,
};

fn bench_portfolio(n: usize) -> Portfolio {
    let sectors = [
        Sector::Technology,
        Sector::Healthcare,
        Sector::Industrials,
        Sector::Consumer,
        Sector::Financials,
        Sector::Energy,
        Sector::RealEstate,
        Sector::Materials,
        Sector::Telecom,
        Sector::Utilities,
    ];
    let geographies = [
        Geography::EuCore,
        Geography::Uk,
        Geography::EuPeriphery,
        Geography::Nordics,
    ];
    let loans: Vec<Loan> = (0..n)
        .map(|i| Loan {
            loan_id: format!("L-{i:04}"),
            issuer: format!("I-{:04}", i / 2),
            sector: sectors[i % sectors.len()],
            geography: geographies[i % geographies.len()],
            seniority: if i % 3 == 0 {
                Seniority::SeniorSecured
            } else {
                Seniority::SeniorUnsecured
            },
            principal: 5_000_000.0 + (i as f64) * 10_000.0,
            coupon: 0.07,
            maturity_years: 5.0,
            leverage: 3.0 + ((i % 7) as f64) * 0.4,
            asset_volatility: 0.30,
            covenants: vec![Covenant {
                name: "net_leverage".into(),
                threshold: 5.5,
                direction: CovenantDirection::LeqThreshold,
                cushion: 0.18,
            }],
            collateral_coverage: 1.3,
        })
        .collect();
    Portfolio {
        firm_id: "F-BENCH".into(),
        as_of: "2026-03-31".into(),
        loans,
    }
}

fn bench_scenario() -> MacroScenario {
    MacroScenario {
        scenario_id: "stagflation_severe".into(),
        name: "Stagflation severe".into(),
        gdp_shock: -0.04,
        rate_shock_bps: 250.0,
        sector_shocks: BTreeMap::new(),
    }
}

fn bench(c: &mut Criterion) {
    let p = bench_portfolio(100);
    let s = bench_scenario();
    let cfg = SimConfig {
        n_paths: 20_000,
        chunk_size: 2_000,
        ..SimConfig::default()
    };
    c.bench_function("simulate_100x20k", |b| {
        b.iter(|| simulate(&p, &s, cfg).unwrap());
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
