"""Coupon distribution from ECB MFI new-business lending rates to NFCs.

Input: ``data/raw/ecb/lending_rates_NFC_new_business.csv`` — euro-area
       weighted-average MFI lending rate on new business loans to NFCs.

Output: ``data/calibrated/coupon_distribution.json`` — base lending rate
        (the index off which a private-credit coupon prices), the
        empirical 10y/3y averages, and a private-credit spread overlay.

Private credit prices at a spread above bank MFI lending — typically
+400 to +700 bps for senior secured mid-market direct lending, more
for second-lien / unitranche / mezz. We bake in a 525 bps central
spread (mid of senior unitranche guidance) and a 200 bps standard
deviation.
"""

from __future__ import annotations

import json

import pandas as pd

from ._paths import RAW, write_json

DEFAULT_SPREAD_BPS = 525.0
SPREAD_STDEV_BPS = 200.0


def calibrate() -> dict:
    path = RAW / "ecb" / "lending_rates_NFC_new_business.csv"
    df = pd.read_csv(path)
    df = df[df["OBS_VALUE"].notna()].copy()
    df["TIME_PERIOD"] = pd.to_datetime(df["TIME_PERIOD"], format="%Y-%m")
    df = df.sort_values("TIME_PERIOD").reset_index(drop=True)

    most_recent = float(df.iloc[-1]["OBS_VALUE"])
    avg_3y = float(df.tail(36)["OBS_VALUE"].mean())
    avg_10y = float(df.tail(120)["OBS_VALUE"].mean())
    full_mean = float(df["OBS_VALUE"].mean())
    full_stdev = float(df["OBS_VALUE"].std())

    # Coupon model: base = max(most_recent, avg_3y) + N(spread, spread_stdev)/10000.
    base_pct = max(most_recent, avg_3y)
    central_coupon = base_pct / 100.0 + DEFAULT_SPREAD_BPS / 10000.0

    out = {
        "version": "v0.1.0-calibrated",
        "generated_from": "data/scripts/calibrate/coupons.py",
        "source": "ECB MFI new-business lending rates to NFCs, euro area",
        "stats_pct": {
            "most_recent": round(most_recent, 2),
            "avg_3y": round(avg_3y, 2),
            "avg_10y": round(avg_10y, 2),
            "full_history_mean": round(full_mean, 2),
            "full_history_stdev": round(full_stdev, 2),
        },
        "base_lending_rate_pct": round(base_pct, 2),
        "private_credit_spread_bps_central": DEFAULT_SPREAD_BPS,
        "private_credit_spread_bps_stdev": SPREAD_STDEV_BPS,
        "central_coupon_fraction": round(central_coupon, 4),
        "_notes": [
            "Base rate is max(most-recent monthly observation, trailing-3y mean)",
            "to avoid under-pricing if rates are below the cycle median.",
            "Private-credit spread of 525 bps is the mid of senior unitranche",
            "guidance from public AIC / Cliffwater commentary; second-lien and",
            "mezz add 200-400 bps. The synthetic generator samples loan-by-loan",
            "from N(spread, stdev).",
        ],
    }
    write_json(out, "coupon_distribution.json")
    return out


if __name__ == "__main__":
    out = calibrate()
    print(json.dumps(out, indent=2, default=str))
