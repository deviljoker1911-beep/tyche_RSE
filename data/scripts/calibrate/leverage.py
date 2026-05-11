"""Leverage distribution from SEC EDGAR Financial Statement Data Sets.

For each US public-company filing in the 2024Q3 + 2024Q4 FSDS bundles, we
compute a debt/EBITDA proxy:

    leverage = (LongTermDebt + DebtCurrent)
             / (OperatingIncomeLoss + DepreciationAndAmortization)

We then summarise the cross-sectional distribution as a set of
percentiles. The synthetic-portfolio generator samples from this
distribution (truncated to a reasonable mid-market band).

European private credit is distinct from US public equity (more
levered, more covenanted, more sector-concentrated), so this is a
**proxy**, not a substitute. The calibration note in the report calls
this out explicitly.

Input:  ``data/processed/sec_edgar/{2024q3,2024q4}/{sub,num}.txt``
Output: ``data/calibrated/leverage_distribution.json``
"""

from __future__ import annotations

from pathlib import Path

import numpy as np
import pandas as pd

from ._paths import PROC, write_json

QUARTERS = ("2024q3", "2024q4")

# XBRL tags we need.
DEBT_TAGS = ("LongTermDebt", "LongTermDebtNoncurrent")
DEBT_CURRENT_TAGS = ("DebtCurrent", "ShortTermBorrowings", "CommercialPaper")
OPINC_TAGS = ("OperatingIncomeLoss",)
DA_TAGS = (
    "DepreciationAndAmortization",
    "DepreciationDepletionAndAmortization",
    "DepreciationAmortizationAndAccretionNet",
)


def _load_quarter(quarter: str) -> pd.DataFrame:
    """Return one row per (cik, tag) with the latest numeric fact."""
    base = PROC / "sec_edgar" / quarter
    sub = pd.read_csv(base / "sub.txt", sep="\t", dtype=str, low_memory=False)
    num = pd.read_csv(base / "num.txt", sep="\t", dtype=str, low_memory=False)
    num["value"] = pd.to_numeric(num["value"], errors="coerce")
    num["ddate"] = pd.to_datetime(num["ddate"], format="%Y%m%d", errors="coerce")
    # Keep only annual / quarterly snapshots (qtrs is "0" for instant facts,
    # "4" for full-year, etc.).
    sub_form = sub[sub["form"].isin(["10-K", "10-Q"])][["adsh", "cik", "name", "sic"]]
    merged = num.merge(sub_form, on="adsh", how="inner")
    return merged


def _pick_latest(df: pd.DataFrame, tags: tuple[str, ...]) -> pd.DataFrame:
    sub = df[df["tag"].isin(tags) & df["value"].notna() & df["ddate"].notna()].copy()
    if sub.empty:
        return pd.DataFrame(columns=["cik", "value"])
    # Latest reporting date per company.
    sub = sub.sort_values("ddate").drop_duplicates(["cik", "tag"], keep="last")
    # When multiple tags map to the same concept, prefer the most-populated one.
    counts = sub["tag"].value_counts()
    sub["tag_rank"] = sub["tag"].map(counts)
    sub = sub.sort_values(["cik", "tag_rank"], ascending=[True, False])
    sub = sub.drop_duplicates("cik", keep="first")
    return sub[["cik", "value"]]


def calibrate() -> dict:
    frames = [_load_quarter(q) for q in QUARTERS]
    big = pd.concat(frames, ignore_index=True)

    debt = _pick_latest(big, DEBT_TAGS).rename(columns={"value": "long_term_debt"})
    short = _pick_latest(big, DEBT_CURRENT_TAGS).rename(columns={"value": "short_term_debt"})
    opinc = _pick_latest(big, OPINC_TAGS).rename(columns={"value": "op_income"})
    da = _pick_latest(big, DA_TAGS).rename(columns={"value": "depreciation"})

    merged = debt.merge(short, on="cik", how="left").merge(opinc, on="cik").merge(da, on="cik")
    merged["short_term_debt"] = merged["short_term_debt"].fillna(0.0)
    merged["depreciation"] = merged["depreciation"].fillna(0.0)

    # EBITDA proxy.
    merged["ebitda"] = merged["op_income"] + merged["depreciation"]
    merged["total_debt"] = merged["long_term_debt"] + merged["short_term_debt"]

    # Drop unusable rows: non-positive EBITDA (negative leverage is meaningless
    # for private credit), or zero debt.
    clean = merged[(merged["ebitda"] > 0) & (merged["total_debt"] > 0)].copy()
    clean["leverage"] = clean["total_debt"] / clean["ebitda"]

    # Clip to a private-credit-plausible band. Beyond 15x is distressed; below
    # 0.1x is essentially debt-free and outside the calibration target.
    band = clean[(clean["leverage"] >= 0.1) & (clean["leverage"] <= 15.0)]

    pcts = [5, 10, 25, 50, 75, 90, 95]
    percentiles = {f"p{p}": float(round(np.percentile(band["leverage"], p), 3)) for p in pcts}

    out = {
        "version": "v0.1.0-calibrated",
        "generated_from": "data/scripts/calibrate/leverage.py",
        "source": "SEC EDGAR Financial Statement Data Sets, 2024Q3+Q4 filings",
        "sample_size": int(len(band)),
        "raw_sample_size": int(len(merged)),
        "metric": "(LongTermDebt + DebtCurrent) / (OperatingIncomeLoss + D&A)",
        "filter": "0.1x <= leverage <= 15x, EBITDA > 0",
        "percentiles": percentiles,
        "mean": float(round(band["leverage"].mean(), 3)),
        "median": float(round(band["leverage"].median(), 3)),
        "stdev": float(round(band["leverage"].std(), 3)),
        "_notes": [
            "This is a US public-company proxy. European mid-market direct",
            "lending is materially more levered: typical range is 4.0–6.5x",
            "vs the ~3.5x median here. The synthetic generator uses these",
            "percentiles as the *shape* of the distribution but rescales the",
            "median to a credit-fund-realistic anchor (default: 4.5x).",
        ],
    }
    write_json(out, "leverage_distribution.json")
    return out


if __name__ == "__main__":
    import json
    out = calibrate()
    print(json.dumps(out, indent=2, default=str))
