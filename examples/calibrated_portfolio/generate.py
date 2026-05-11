#!/usr/bin/env python3
"""Empirically-calibrated European mid-market direct-lending book.

Reads the JSON artifacts under ``data/calibrated/`` and samples a
synthetic 50-loan portfolio whose distribution properties match the
public-data priors. Run after ``python3 -m data.scripts.calibrate``.

    python3 examples/calibrated_portfolio/generate.py
"""

from __future__ import annotations

import json
import random
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
CAL = ROOT / "data" / "calibrated"
OUT = ROOT / "examples" / "calibrated_portfolio"
N = 50

GEOS = ["uk", "eu_core", "eu_periphery", "nordics", "switzerland"]
GEO_WEIGHTS = [0.18, 0.42, 0.18, 0.14, 0.08]
SENIORITIES = ["senior_secured", "second_lien", "senior_unsecured", "subordinated", "mezzanine"]
SENIORITY_WEIGHTS = [0.62, 0.14, 0.16, 0.05, 0.03]


def _load(name: str) -> dict:
    return json.loads((CAL / name).read_text())


def _sample_sector(rng: random.Random, weights: dict[str, float]) -> str:
    sectors = list(weights.keys())
    probs = list(weights.values())
    return rng.choices(sectors, weights=probs, k=1)[0]


def _sample_leverage(rng: np.random.Generator, pcts: dict[str, float], target_median: float = 4.5) -> float:
    pts_q = [5, 10, 25, 50, 75, 90, 95]
    qs = np.array([p / 100 for p in pts_q])
    vs = np.array([pcts[f"p{p}"] for p in pts_q])
    scale = target_median / vs[3]
    vs = vs * scale
    u = rng.uniform(qs.min(), qs.max())
    return float(np.interp(u, qs, vs))


def _sample_coupon(rng: np.random.Generator, base_pct: float, spread_central_bps: float,
                   spread_stdev_bps: float) -> float:
    # base + spread, expressed as a fraction (0.0925 = 9.25%).
    spread_bps = rng.normal(spread_central_bps, spread_stdev_bps)
    return round((base_pct / 100.0) + max(spread_bps, 100.0) / 10_000.0, 4)


def generate() -> None:
    sectors = _load("sector_weights.json")["weights"]
    leverage = _load("leverage_distribution.json")["percentiles"]
    coupons = _load("coupon_distribution.json")
    pd_cal = _load("pd_anchor.json")

    base_pct = coupons["base_lending_rate_pct"]
    spread_central = coupons["private_credit_spread_bps_central"]
    spread_stdev = coupons["private_credit_spread_bps_stdev"]

    rng = random.Random(0x7CECA1)
    nprng = np.random.default_rng(0x7CECA1B6)

    loans = []
    for i in range(N):
        sector = _sample_sector(rng, sectors) if sectors else "industrials"
        geo = rng.choices(GEOS, weights=GEO_WEIGHTS)[0]
        seniority = rng.choices(SENIORITIES, weights=SENIORITY_WEIGHTS)[0]
        principal = round(rng.uniform(10_000_000, 80_000_000), 2)
        coupon = _sample_coupon(nprng, base_pct, spread_central, spread_stdev)
        maturity = round(rng.uniform(3.0, 7.0), 2)
        leverage_x = round(_sample_leverage(nprng, leverage), 2)
        vol = round(rng.uniform(0.25, 0.40), 3)
        cushion = round(rng.uniform(0.08, 0.32), 3)
        cc = round(rng.uniform(0.8, 1.6), 3)
        loans.append({
            "loan_id": f"L-{i+1:03d}",
            "issuer": f"I-{(i // 2) + 1:03d}",
            "sector": sector,
            "geography": geo,
            "seniority": seniority,
            "principal": principal,
            "coupon": coupon,
            "maturity_years": maturity,
            "leverage": leverage_x,
            "asset_volatility": vol,
            "covenants": [{
                "name": "net_leverage",
                "threshold": 5.5,
                "direction": "leq_threshold",
                "cushion": cushion,
            }],
            "collateral_coverage": cc,
        })

    portfolio = {
        "firm_id": "F-AURIC-CAPITAL-CALIBRATED",
        "as_of": "2026-03-31",
        "loans": loans,
        "_calibration": {
            "sector_source": "Eurostat NACE A64 EU27 2022 GVA",
            "leverage_source": "SEC EDGAR FSDS 2024Q3+Q4",
            "coupon_source": "ECB MFI new-business lending rates to NFCs",
            "pd_anchor_recommended": pd_cal["calibrated_pd_anchor"],
            "pd_anchor_spike_current": pd_cal["current_pd_anchor"],
        },
    }
    (OUT / "portfolio.json").write_text(json.dumps(portfolio, indent=2))

    # Copy scenarios with provenance stripped to match the simulator's
    # MacroScenario JSON shape.
    scenarios = _load("historical_scenarios.json")["scenarios"]
    clean = [{k: v for k, v in s.items() if not k.startswith("_")} for s in scenarios]
    (OUT / "scenarios.json").write_text(json.dumps(clean, indent=2))

    print(f"wrote {OUT / 'portfolio.json'} with {N} loans")
    print(f"  median leverage: "
          f"{np.median([l['leverage'] for l in loans]):.2f}x")
    print(f"  median coupon: "
          f"{100 * np.median([l['coupon'] for l in loans]):.2f}%")
    print(f"  sectors:")
    from collections import Counter
    for k, v in Counter(l["sector"] for l in loans).most_common():
        print(f"    {k:<12}  {v:>2}")
    print(f"wrote {OUT / 'scenarios.json'} with {len(clean)} scenarios")


if __name__ == "__main__":
    generate()
