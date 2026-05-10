#!/usr/bin/env python3
"""Generate the synthetic European mid-market direct-lending portfolio.

Run from the repo root:

    python3 examples/synthetic_portfolio/generate.py

The script is deterministic (seeded RNG) so the produced JSON is stable across
machines.
"""

from __future__ import annotations

import json
import random
from pathlib import Path

SECTORS = [
    "technology", "healthcare", "industrials", "consumer", "financials",
    "energy", "real_estate", "materials", "telecom", "utilities",
]
GEOS = ["uk", "eu_core", "eu_periphery", "nordics", "switzerland"]
SENIORITY = ["senior_secured", "second_lien", "senior_unsecured", "subordinated", "mezzanine"]
SENIORITY_WEIGHTS = [0.55, 0.15, 0.18, 0.07, 0.05]

OUT = Path(__file__).parent
N = 50


def gen() -> None:
    rng = random.Random(0x7CE_ACE)  # stable seed
    loans = []
    for i in range(N):
        sector = rng.choice(SECTORS)
        geo = rng.choices(GEOS, weights=[0.18, 0.42, 0.18, 0.14, 0.08])[0]
        seniority = rng.choices(SENIORITY, weights=SENIORITY_WEIGHTS)[0]
        principal = round(rng.uniform(8_000_000, 95_000_000), 2)
        coupon = round(rng.uniform(0.06, 0.115), 4)
        maturity = round(rng.uniform(2.0, 7.5), 2)
        leverage = round(rng.uniform(2.5, 6.5), 2)
        vol = round(rng.uniform(0.22, 0.42), 3)
        cushion = round(rng.uniform(0.08, 0.32), 3)
        cc = round(rng.uniform(0.7, 1.7), 3)
        loans.append({
            "loan_id": f"L-{i+1:03d}",
            "issuer": f"I-{(i // 2) + 1:03d}",
            "sector": sector,
            "geography": geo,
            "seniority": seniority,
            "principal": principal,
            "coupon": coupon,
            "maturity_years": maturity,
            "leverage": leverage,
            "asset_volatility": vol,
            "covenants": [
                {
                    "name": "net_leverage",
                    "threshold": 5.5,
                    "direction": "leq_threshold",
                    "cushion": cushion,
                }
            ],
            "collateral_coverage": cc,
        })
    portfolio = {
        "firm_id": "F-AURIC-CAPITAL",
        "as_of": "2026-03-31",
        "loans": loans,
    }
    (OUT / "portfolio.json").write_text(json.dumps(portfolio, indent=2))

    scenarios = [
        {
            "scenario_id": "baseline_2026",
            "name": "Baseline 2026",
            "gdp_shock": -0.005,
            "rate_shock_bps": 0.0,
            "sector_shocks": {},
        },
        {
            "scenario_id": "stagflation_severe",
            "name": "Stagflation severe",
            "gdp_shock": -0.04,
            "rate_shock_bps": 250.0,
            "sector_shocks": {
                "energy": {"ebitda_shock": -0.18, "asset_shock": -0.10},
                "consumer": {"ebitda_shock": -0.10, "asset_shock": -0.06},
                "real_estate": {"ebitda_shock": -0.12, "asset_shock": -0.20},
            },
        },
        {
            "scenario_id": "climate_transition_2030",
            "name": "Climate transition 2030",
            "gdp_shock": -0.015,
            "rate_shock_bps": 50.0,
            "sector_shocks": {
                "energy": {"ebitda_shock": -0.30, "asset_shock": -0.40},
                "materials": {"ebitda_shock": -0.12, "asset_shock": -0.10},
                "utilities": {"ebitda_shock": -0.05, "asset_shock": 0.05},
                "technology": {"ebitda_shock": 0.02, "asset_shock": 0.0},
            },
        },
        {
            "scenario_id": "geopolitical_eu_break",
            "name": "Geopolitical: EU fragmentation",
            "gdp_shock": -0.06,
            "rate_shock_bps": 350.0,
            "sector_shocks": {
                "financials": {"ebitda_shock": -0.20, "asset_shock": -0.15},
                "industrials": {"ebitda_shock": -0.18, "asset_shock": -0.10},
                "telecom": {"ebitda_shock": -0.05, "asset_shock": -0.05},
            },
        },
    ]
    (OUT / "scenarios.json").write_text(json.dumps(scenarios, indent=2))
    print(f"wrote {OUT / 'portfolio.json'} with {N} loans")
    print(f"wrote {OUT / 'scenarios.json'} with {len(scenarios)} scenarios")


if __name__ == "__main__":
    gen()
