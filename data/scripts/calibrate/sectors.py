"""Empirical Tyche-sector weights from Eurostat NACE A64 GVA.

Input: ``data/processed/eurostat/nama_10_a64.csv``
       (single-letter NACE codes, EU27_2020, real GVA in chain-linked €).

Output: ``data/calibrated/sector_weights.json`` — a JSON dict
        ``{ tyche_sector: weight }`` summing to 1.0, restricted to
        private-credit-relevant sectors (we drop public administration,
        education, household services, and extra-territorial bodies).

The mapping NACE A64 → Tyche sector is documented inline. It is the
single point where this calibration depends on a human judgment call,
so it lives in code and is reviewable.
"""

from __future__ import annotations

import pandas as pd

from ._paths import PROC, write_json

# NACE A21 letter → Tyche sector. NACE codes lifted from Eurostat A64 doc.
#   A   Agriculture, forestry and fishing
#   B   Mining and quarrying
#   C   Manufacturing
#   D   Electricity, gas, steam, air-conditioning
#   E   Water supply, sewerage, waste management
#   F   Construction
#   G   Wholesale and retail trade
#   H   Transportation and storage
#   I   Accommodation and food
#   J   Information and communication
#   K   Financial and insurance
#   L   Real estate
#   M   Professional, scientific, technical
#   N   Administrative and support services
#   O   Public administration                  ← excluded (not private credit)
#   P   Education                              ← excluded
#   Q   Human health and social work
#   R   Arts, entertainment, recreation
#   S   Other service activities
#   T   Activities of households                ← excluded
#   U   Extra-territorial organisations         ← excluded
NACE_TO_TYCHE: dict[str, str] = {
    "A": "other",
    "B": "materials",
    "C": "industrials",
    "D": "utilities",
    "E": "utilities",
    "F": "industrials",
    "G": "consumer",
    "H": "industrials",
    "I": "consumer",
    "J": "telecom",            # info+comm dominated by telcos in EU NACE J
    "K": "financials",
    "L": "real_estate",
    "M": "other",
    "N": "other",
    "Q": "healthcare",
    "R": "consumer",
    "S": "other",
}
EXCLUDED = {"O", "P", "T", "U"}

# Tyche's full sector enum — we want every sector to appear, with 0 weight
# if no NACE letter maps to it.
TYCHE_SECTORS = [
    "technology", "healthcare", "industrials", "consumer", "financials",
    "energy", "real_estate", "materials", "telecom", "utilities", "other",
]


def calibrate(year: int = 2022, geo: str = "EU27_2020") -> dict:
    """Compute empirical Tyche-sector weights for ``geo`` and ``year``."""
    path = PROC / "eurostat" / "nama_10_a64.csv"
    df = pd.read_csv(path, low_memory=False)

    mask = (
        (df["na_item"] == "B1G")           # Gross value added
        & (df["unit"] == "CLV05_MEUR")     # Chain-linked volumes, base 2005, EUR
        & (df["geo"] == geo)
        & (df["TIME_PERIOD"] == year)
        & df["nace_r2"].str.fullmatch(r"[A-U]")
    )
    sub = df.loc[mask, ["nace_r2", "OBS_VALUE"]].copy()
    if sub.empty:
        raise RuntimeError(f"no rows for geo={geo} year={year}")

    sub = sub[~sub["nace_r2"].isin(EXCLUDED)]
    sub["tyche"] = sub["nace_r2"].map(NACE_TO_TYCHE)
    if sub["tyche"].isna().any():
        unmapped = sorted(sub.loc[sub["tyche"].isna(), "nace_r2"].unique())
        raise RuntimeError(f"NACE letters not mapped: {unmapped}")

    weights = sub.groupby("tyche")["OBS_VALUE"].sum()
    weights = weights / weights.sum()

    # Initialise every Tyche sector at zero, then overlay the empirical share.
    full = {s: 0.0 for s in TYCHE_SECTORS}
    for sector, w in weights.items():
        full[sector] = round(float(w), 4)

    out = {
        "version": "v0.1.0-calibrated",
        "generated_from": "data/scripts/calibrate/sectors.py",
        "source": "Eurostat NACE A64 gross value added (chain-linked € 2005)",
        "geo": geo,
        "year": year,
        "weights": full,
        "_notes": [
            "Public administration (O), education (P), household activities (T),",
            "and extra-territorial organisations (U) are excluded — private credit",
            "does not lend to these sectors at meaningful scale.",
            "NACE J (information+communication) is mapped to telecom; in a",
            "European mid-market portfolio J is dominated by telcos. Pure tech",
            "lending is captured separately via the 'technology' bucket (which",
            "remains 0 here because Eurostat A21 does not split it from J).",
        ],
    }
    write_json(out, "sector_weights.json")
    return out


if __name__ == "__main__":
    import json
    out = calibrate()
    print(json.dumps(out, indent=2, default=str))
