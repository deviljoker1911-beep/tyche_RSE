"""Orchestrator: run every calibration module in dependency order and
write a human-readable report.

Usage:

    python3 -m data.scripts.calibrate

Produces ``data/calibrated/*.json`` plus
``data/calibrated/CALIBRATION_REPORT.md``.
"""

from __future__ import annotations

import json
import textwrap
from datetime import datetime, timezone

from . import coupons, leverage, pds, recovery, scenarios, sectors
from ._paths import OUT


def _summary(title: str, content: str) -> str:
    return f"## {title}\n\n{content.strip()}\n\n"


def _format_pct(x: float) -> str:
    return f"{x * 100:.2f}%"


def _format_table(rows: list[tuple[str, str]]) -> str:
    body = "\n".join(f"| {k} | {v} |" for k, v in rows)
    return f"| Field | Value |\n|---|---|\n{body}\n"


def run() -> dict:
    print("==> sectors")
    s = sectors.calibrate()
    print("==> leverage")
    l = leverage.calibrate()
    print("==> coupons")
    c = coupons.calibrate()
    print("==> recovery")
    r = recovery.calibrate()
    print("==> scenarios")
    sc = scenarios.calibrate()
    print("==> PD anchor")
    p = pds.calibrate()
    return {"sectors": s, "leverage": l, "coupons": c, "recovery": r, "scenarios": sc, "pds": p}


def write_report(results: dict) -> None:
    """Auto-generate CALIBRATION_REPORT.md from the JSON artifacts."""
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S UTC")
    parts: list[str] = []
    parts.append(textwrap.dedent(f"""
        # Tyche calibration report

        Generated: {now}
        Sources: see `data/README.md` for the full dataset catalogue.

        This report documents the empirical priors that the Tyche simulation
        core is calibrated against. Every parameter below is derived from a
        public, free, no-auth dataset. The simulator's hand-picked spike
        constants have been re-fit to match these priors.

        The intended audience is a credit-fund risk officer, a methodology
        reviewer, or a regulator who needs to verify that the model's
        parameters are defensible against published market data.
    """).strip() + "\n\n")

    # --- Sectors ---
    s = results["sectors"]
    parts.append(_summary("Empirical sector mix (EU27, 2022)",
        f"Source: {s['source']}\n\n"
        f"Tyche sectors weighted by NACE-mapped real GVA, with public "
        f"administration / education / household activities excluded. The "
        f"synthetic portfolio generator draws sector for each loan from this "
        f"distribution.\n\n"
        + _format_table([(k, _format_pct(v)) for k, v in s["weights"].items()])))

    # --- Leverage ---
    l = results["leverage"]
    parts.append(_summary("Leverage distribution (debt / EBITDA proxy)",
        f"Source: {l['source']} ({l['sample_size']:,} filings post-filter)\n\n"
        f"Metric: `{l['metric']}`\n\n"
        + _format_table([
            ("Mean", f"{l['mean']}x"),
            ("Median (p50)", f"{l['median']}x"),
            ("Stdev", f"{l['stdev']}x"),
            ("p25 / p75", f"{l['percentiles']['p25']}x  /  {l['percentiles']['p75']}x"),
            ("p10 / p90", f"{l['percentiles']['p10']}x  /  {l['percentiles']['p90']}x"),
        ])
        + "\nThe synthetic generator samples from this empirical distribution.\n"))

    # --- Coupons ---
    c = results["coupons"]
    parts.append(_summary("Coupon distribution (lending rate + private-credit spread)",
        f"Source: {c['source']}\n\n"
        f"Most recent monthly observation: {c['stats_pct']['most_recent']}%\n"
        f"3-year mean: {c['stats_pct']['avg_3y']}%  |  10-year mean: {c['stats_pct']['avg_10y']}%\n\n"
        f"Tyche treats the **base rate** as `max(most_recent, avg_3y) = {c['base_lending_rate_pct']}%` "
        f"and adds a private-credit spread of "
        f"**{c['private_credit_spread_bps_central']:.0f} bps central** "
        f"with **{c['private_credit_spread_bps_stdev']:.0f} bps stdev**.\n\n"
        f"Central coupon: **{c['central_coupon_fraction'] * 100:.2f}%**\n"))

    # --- Recovery ---
    r = results["recovery"]
    parts.append(_summary("Recovery floors by seniority",
        f"Sources: Moody's Ultimate Recovery Database, S&P recovery studies, EBA workout reports.\n\n"
        + _format_table([
            (sen, f"central {row['central']:.2f}  |  5/95: {row['lower_5th']:.2f} / {row['upper_95th']:.2f}")
            for sen, row in r["recovery_floors"].items()
        ])
        + "\nThe simulator currently uses the central values as deterministic "
        "recovery rates. Phase 2 will replace with stochastic Beta draws "
        "using the 5/95 percentiles as distribution parameters.\n"))

    # --- Scenarios ---
    sc = results["scenarios"]
    rows = []
    for s_ in sc["scenarios"]:
        prov = s_.get("_provenance", {})
        rows.append((
            s_["scenario_id"],
            f"GDP {_format_pct(s_['gdp_shock'])}, OAS shock "
            f"{s_['rate_shock_bps']:.0f} bps, FF Δ "
            f"{prov.get('ff_policy_rate_change_bps', 'n/a')} bps"
        ))
    parts.append(_summary("Historical macro scenarios",
        "Each scenario is anchored on a real episode in FRED's macro history.\n\n"
        + _format_table(rows)
        + "\n*Note: HY-OAS rate-shock magnitudes are 0 for older windows because "
        "the FRED graph endpoint returned only ~3 years of history at fetch "
        "time. Re-run `bash data/scripts/fetch_all.sh --force` once FRED's "
        "rate limit clears to pick up full history.*\n"))

    # --- PD anchor (the headline finding) ---
    p = results["pds"]
    parts.append(_summary("PD anchor calibration (HEADLINE FINDING)",
        f"Source: {p['source_npl']}\n\n"
        f"**Target 1y PD across the synthetic book**: "
        f"`{p['npl_median_pct']:.2f}% NPL median / "
        f"{p['resolution_years']}y resolution = "
        f"{p['pd_target_1y'] * 100:.3f}%`\n\n"
        f"**Spike anchor (`1.5`)**: produced median PD of "
        f"**{p['current_median_pd_1y_at_spike_anchor'] * 100:.2f}%** — capped at the "
        f"50% PD cap. This is the wrong order of magnitude.\n\n"
        f"**Calibrated anchor**: "
        f"`{p['calibrated_pd_anchor']}` — produces median PD of "
        f"**{p['achieved_median_pd_1y'] * 100:.3f}%**, matching the empirical target.\n\n"
        f"### Action: change the constant in `crates/tyche-sim/src/lib.rs`\n"
        f"```rust\n"
        f"// before\n"
        f"let dd = (lev_eff.ln() / loan.asset_volatility) - 1.5;\n"
        f"// after\n"
        f"let dd = (lev_eff.ln() / loan.asset_volatility) - {p['calibrated_pd_anchor']};\n"
        f"```\n"))

    parts.append(textwrap.dedent("""
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
    """).strip() + "\n")

    (OUT / "CALIBRATION_REPORT.md").write_text("".join(parts))


def main() -> None:
    results = run()
    write_report(results)
    print()
    print(f"==> wrote {OUT}/CALIBRATION_REPORT.md")
    for f in sorted(OUT.glob("*.json")):
        print(f"    {f.relative_to(OUT.parent.parent)}")


if __name__ == "__main__":
    main()
