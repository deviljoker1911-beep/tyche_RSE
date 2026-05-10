//! `tyche` — command-line driver for the Tyche stack.
//!
//! See `--help` for usage. This binary is the canonical reference: the WASM
//! and Python bindings expose strict subsets of the same surface.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;
use rand::RngCore;
use rand::rngs::OsRng;
use serde::Serialize;
use tyche_attest::{AttestBatch, AttestationRecord, build_attestation, verify_attestation};
use tyche_fed::{aggregate, verify_aggregate_consistency};
use tyche_sim::{SimConfig, simulate};
use tyche_types::{MacroScenario, Portfolio, RiskMetrics};

#[derive(Parser, Debug)]
#[command(
    name = "tyche",
    version,
    about = "Federated risk simulation for European private credit",
    long_about = None
)]
struct Cli {
    /// Output everything as JSON. Useful for scripting.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run a Monte Carlo simulation against a scenario.
    Simulate(SimulateArgs),
    /// Run a simulation and produce a signed attestation record.
    Attest(AttestArgs),
    /// Verify the signature on an attestation record.
    Verify(VerifyArgs),
    /// Aggregate a directory of contributions into a federation root.
    FederateAggregate(AggregateArgs),
    /// Run a fixed benchmark portfolio and report timing.
    Bench,
}

#[derive(clap::Args, Debug)]
struct SimulateArgs {
    /// Path to the portfolio JSON file.
    #[arg(long)]
    portfolio: PathBuf,
    /// Scenario id (must match an entry in the scenarios file).
    #[arg(long)]
    scenario: String,
    /// Path to the scenarios file (a JSON array of `MacroScenario`).
    #[arg(long, default_value = "examples/synthetic_portfolio/scenarios.json")]
    scenarios_file: PathBuf,
    /// Number of Monte Carlo paths.
    #[arg(long, default_value_t = 20_000)]
    paths: u64,
    /// Reproducibility seed.
    #[arg(long)]
    seed: Option<u64>,
}

#[derive(clap::Args, Debug)]
struct AttestArgs {
    /// Path to the portfolio JSON file.
    #[arg(long)]
    portfolio: PathBuf,
    /// Scenario id.
    #[arg(long)]
    scenario: String,
    /// Path to the scenarios file.
    #[arg(long, default_value = "examples/synthetic_portfolio/scenarios.json")]
    scenarios_file: PathBuf,
    /// Number of paths.
    #[arg(long, default_value_t = 20_000)]
    paths: u64,
    /// Reproducibility seed.
    #[arg(long)]
    seed: Option<u64>,
    /// Path to a 32-byte raw Ed25519 secret key. If absent, an ephemeral key
    /// is generated and the public key is printed.
    #[arg(long)]
    signer_key: Option<PathBuf>,
    /// Where to write the resulting attestation record JSON.
    #[arg(long, default_value = "out/record.json")]
    out: PathBuf,
    /// Model version string included in the record. Bump on breaking changes.
    #[arg(long, default_value = "v0.1.0-spike")]
    model_version: String,
}

#[derive(clap::Args, Debug)]
struct VerifyArgs {
    /// Path to the attestation record JSON.
    #[arg(long)]
    record: PathBuf,
}

#[derive(clap::Args, Debug)]
struct AggregateArgs {
    /// Directory of `*.json` contribution files (`tyche_fed::Contribution`).
    #[arg(long)]
    inputs: PathBuf,
    /// Where to write the aggregate root JSON.
    #[arg(long, default_value = "out/aggregate.json")]
    out: PathBuf,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Simulate(args) => run_simulate(&args, cli.json),
        Cmd::Attest(args) => run_attest(&args, cli.json),
        Cmd::Verify(args) => run_verify(&args, cli.json),
        Cmd::FederateAggregate(args) => run_aggregate(&args, cli.json),
        Cmd::Bench => run_bench(cli.json),
    }
}

fn load_portfolio(path: &Path) -> Result<Portfolio> {
    let bytes = fs::read(path).with_context(|| format!("read portfolio {}", path.display()))?;
    let p: Portfolio = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse portfolio {}", path.display()))?;
    p.validate().map_err(|e| anyhow!("portfolio invalid: {e}"))?;
    Ok(p)
}

fn load_scenario(path: &Path, id: &str) -> Result<MacroScenario> {
    let bytes = fs::read(path).with_context(|| format!("read scenarios {}", path.display()))?;
    let scenarios: Vec<MacroScenario> = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse scenarios {}", path.display()))?;
    scenarios
        .into_iter()
        .find(|s| s.scenario_id == id)
        .ok_or_else(|| anyhow!("no scenario with id `{id}` in {}", path.display()))
}

fn make_config(paths: u64, seed: Option<u64>) -> SimConfig {
    SimConfig {
        n_paths: paths,
        seed: seed.unwrap_or_else(|| SimConfig::default().seed),
        ..SimConfig::default()
    }
}

fn run_simulate(args: &SimulateArgs, json: bool) -> Result<()> {
    let portfolio = load_portfolio(&args.portfolio)?;
    let scenario = load_scenario(&args.scenarios_file, &args.scenario)?;
    let cfg = make_config(args.paths, args.seed);
    let metrics = simulate(&portfolio, &scenario, cfg)?;
    print_metrics(&metrics, &portfolio, &scenario, json)
}

fn run_attest(args: &AttestArgs, json: bool) -> Result<()> {
    let portfolio = load_portfolio(&args.portfolio)?;
    let scenario = load_scenario(&args.scenarios_file, &args.scenario)?;
    let cfg = make_config(args.paths, args.seed);
    let metrics = simulate(&portfolio, &scenario, cfg)?;

    let signer = match &args.signer_key {
        Some(path) => {
            let bytes = fs::read(path).with_context(|| format!("read key {}", path.display()))?;
            let arr: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| anyhow!("signer key must be exactly 32 bytes"))?;
            SigningKey::from_bytes(&arr)
        }
        None => SigningKey::generate(&mut OsRng),
    };

    let mut blinding = [0u8; 32];
    OsRng.fill_bytes(&mut blinding);

    let record = build_attestation(
        &portfolio,
        &blinding,
        &scenario,
        &metrics,
        &args.model_version,
        &signer,
    )?;

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&args.out, serde_json::to_vec_pretty(&record)?)?;

    if json {
        println!("{}", serde_json::to_string(&record)?);
    } else {
        println!("attestation written: {}", args.out.display());
        println!("  signer pubkey : {}", record.signer_pubkey);
        println!("  input hash    : {}", record.input_hash);
        println!("  result hash   : {}", record.result_hash);
        if args.signer_key.is_none() {
            println!(
                "  ephemeral sk  : {}  (developer only — do not reuse in production)",
                hex::encode(signer.to_bytes())
            );
        }
    }
    Ok(())
}

fn run_verify(args: &VerifyArgs, json: bool) -> Result<()> {
    let bytes = fs::read(&args.record)?;
    let record: AttestationRecord = serde_json::from_slice(&bytes)?;
    match verify_attestation(&record) {
        Ok(()) => {
            if json {
                println!("{{\"verified\":true}}");
            } else {
                println!("OK: signature verifies for {}", record.scenario_id);
            }
            Ok(())
        }
        Err(e) => {
            if json {
                println!("{{\"verified\":false,\"error\":\"{e}\"}}");
            } else {
                eprintln!("FAIL: {e}");
            }
            Err(anyhow!("attestation did not verify"))
        }
    }
}

fn run_aggregate(args: &AggregateArgs, json: bool) -> Result<()> {
    let mut contributions = Vec::new();
    for entry in fs::read_dir(&args.inputs)? {
        let entry = entry?;
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read(entry.path())?;
        let c: tyche_fed::Contribution = serde_json::from_slice(&bytes)?;
        contributions.push(c);
    }
    verify_aggregate_consistency(&contributions)?;
    let root = aggregate(&contributions)?;
    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&args.out, serde_json::to_vec_pretty(&root)?)?;
    if json {
        println!("{}", serde_json::to_string(&root)?);
    } else {
        println!("aggregate root written: {}", args.out.display());
        println!("  metric       : {}", root.metric_name);
        println!("  contributors : {}", root.contributors.len());
    }
    Ok(())
}

fn run_bench(json: bool) -> Result<()> {
    use std::time::Instant;
    let portfolio = bench_portfolio();
    let scenario = MacroScenario {
        scenario_id: "bench".into(),
        name: "Bench".into(),
        gdp_shock: -0.04,
        rate_shock_bps: 250.0,
        sector_shocks: BTreeMap::new(),
    };
    let cfg = SimConfig {
        n_paths: 20_000,
        chunk_size: 2_000,
        ..SimConfig::default()
    };
    let t0 = Instant::now();
    let metrics = simulate(&portfolio, &scenario, cfg)?;
    let dur = t0.elapsed();
    if json {
        #[derive(Serialize)]
        struct Out {
            duration_ms: f64,
            paths_per_sec: f64,
            metrics: RiskMetrics,
        }
        let out = Out {
            duration_ms: dur.as_secs_f64() * 1_000.0,
            paths_per_sec: f64::from(20_000) / dur.as_secs_f64(),
            metrics,
        };
        println!("{}", serde_json::to_string(&out)?);
    } else {
        println!("simulate(100x20k) -> {:.2} ms", dur.as_secs_f64() * 1_000.0);
        println!("  expected_loss : {:.2}", metrics.expected_loss);
        println!("  var_99        : {:.2}", metrics.var_99);
        println!("  es_975        : {:.2}", metrics.expected_shortfall_975);
    }
    Ok(())
}

fn bench_portfolio() -> Portfolio {
    use tyche_types::{Covenant, CovenantDirection, Geography, Loan, Sector, Seniority};
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
    let geos = [Geography::EuCore, Geography::Uk, Geography::EuPeriphery];
    let loans: Vec<Loan> = (0..100)
        .map(|i| Loan {
            loan_id: format!("L-{i:04}"),
            issuer: format!("I-{:04}", i / 2),
            sector: sectors[i % sectors.len()],
            geography: geos[i % geos.len()],
            seniority: Seniority::SeniorSecured,
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

fn print_metrics(
    m: &RiskMetrics,
    portfolio: &Portfolio,
    scenario: &MacroScenario,
    json: bool,
) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string(m)?);
        return Ok(());
    }
    println!("Tyche simulation");
    println!("  firm        : {}", portfolio.firm_id);
    println!("  as_of       : {}", portfolio.as_of);
    println!("  loans       : {}", portfolio.loans.len());
    println!("  exposure    : {:>15.0}", portfolio.total_exposure());
    println!("  scenario    : {}  (gdp_shock {:+.1}%)", scenario.scenario_id, scenario.gdp_shock * 100.0);
    println!("  paths       : {}", m.n_paths);
    println!();
    println!("  Expected loss        : {:>15.2}", m.expected_loss);
    println!("  VaR 95%              : {:>15.2}", m.var_95);
    println!("  VaR 99%              : {:>15.2}", m.var_99);
    println!("  Expected shortfall   : {:>15.2}", m.expected_shortfall_975);
    println!();
    println!("  By sector:");
    for (sector, c) in &m.sector_contributions {
        println!(
            "    {:<14}  EL {:>12.2}   share {:>5.1}%",
            format!("{sector:?}"),
            c.expected_loss,
            c.share * 100.0
        );
    }
    Ok(())
}
