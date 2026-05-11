"""Calibrate Tyche `MacroScenario`s against real historical episodes.

For each named episode (Lehman 2008, COVID 2020, EU sovereign 2011-12,
2022 inflation shock) we extract from the FRED time series:

- ``gdp_shock``        — peak-to-trough drop in real GDP within the window
- ``rate_shock_bps``   — peak widening of HY OAS within the window
- a window definition  — the actual calendar dates the scenario fired

Output: ``data/calibrated/historical_scenarios.json`` — a JSON array of
``MacroScenario`` records ready to be loaded by ``tyche-cli simulate
--scenarios-file ...``.

The sector-shock channel is left empty here; it is populated by
``calibrate.sector_shocks``, which uses Eurostat sectoral GVA covered
by the same windows.
"""

from __future__ import annotations

from dataclasses import dataclass

import pandas as pd

from ._paths import RAW, write_json


@dataclass
class Window:
    scenario_id: str
    name: str
    start: str  # ISO yyyy-mm-dd
    end: str

    def slice(self, df: pd.DataFrame, date_col: str = "observation_date") -> pd.DataFrame:
        m = (df[date_col] >= pd.Timestamp(self.start)) & (df[date_col] <= pd.Timestamp(self.end))
        return df.loc[m].copy()


# Hand-picked windows. Anchored on FRED USREC + market events.
WINDOWS: list[Window] = [
    Window("lehman_2008",          "Global financial crisis",        "2008-06-01", "2009-06-30"),
    Window("eu_sovereign_2011",    "EU sovereign-debt crisis",       "2011-06-01", "2012-12-31"),
    Window("covid_2020",           "COVID demand collapse",          "2020-02-01", "2020-09-30"),
    Window("inflation_shock_2022", "Inflation / rate-shock",         "2022-03-01", "2023-06-30"),
]


def _read_fred(series_id: str) -> pd.DataFrame:
    path = RAW / "fred" / f"{series_id}.csv"
    df = pd.read_csv(path)
    df.columns = [c.lower() for c in df.columns]
    if "observation_date" not in df.columns:
        # FRED occasionally returns "DATE" instead of "observation_date"
        df = df.rename(columns={"date": "observation_date"})
    df["observation_date"] = pd.to_datetime(df["observation_date"])
    val_col = [c for c in df.columns if c != "observation_date"][0]
    df[val_col] = pd.to_numeric(df[val_col], errors="coerce")
    return df.rename(columns={val_col: "value"})


def gdp_shock(window: Window) -> float:
    """Peak-to-trough log change in real GDP across the window."""
    gdp = _read_fred("GDPC1")
    sub = window.slice(gdp).dropna()
    if sub.empty:
        return 0.0
    peak = float(sub["value"].max())
    trough = float(sub["value"].min())
    if peak <= 0:
        return 0.0
    # Negative means contraction. Return as a fractional change.
    return (trough - peak) / peak


def rate_shock_bps(window: Window) -> float:
    """Maximum HY-OAS widening within the window minus the pre-window baseline.

    Falls back to the absolute peak in the window if no pre-window data.
    """
    oas = _read_fred("BAMLH0A0HYM2")
    sub = window.slice(oas).dropna()
    if sub.empty:
        return 0.0
    pre_start = (pd.Timestamp(window.start) - pd.DateOffset(months=6)).strftime("%Y-%m-%d")
    pre = oas[(oas["observation_date"] >= pd.Timestamp(pre_start))
              & (oas["observation_date"] < pd.Timestamp(window.start))]
    baseline = float(pre["value"].mean()) if not pre.empty else float(sub["value"].iloc[0])
    peak = float(sub["value"].max())
    # FRED reports OAS in percent (e.g. 8.5 = 850 bps). Convert to bps.
    return (peak - baseline) * 100.0


def policy_rate_shock_bps(window: Window) -> float:
    """Change in fed-funds policy rate across the window, in basis points."""
    dff = _read_fred("DFF")
    sub = window.slice(dff).dropna()
    if sub.empty:
        return 0.0
    return (float(sub["value"].iloc[-1]) - float(sub["value"].iloc[0])) * 100.0


def calibrate() -> dict:
    """Build the calibrated scenario set."""
    scenarios = []
    for w in WINDOWS:
        gdp = round(gdp_shock(w), 4)
        oas = round(rate_shock_bps(w), 0)
        ff_delta = round(policy_rate_shock_bps(w), 0)
        scenarios.append({
            "scenario_id": w.scenario_id,
            "name": w.name,
            "gdp_shock": gdp,
            "rate_shock_bps": oas,
            "sector_shocks": {},
            # provenance — bookkeeping for the calibration report
            "_provenance": {
                "window_start": w.start,
                "window_end": w.end,
                "ff_policy_rate_change_bps": ff_delta,
                "sources": ["fred:GDPC1", "fred:BAMLH0A0HYM2", "fred:DFF"],
            },
        })

    # Always include a 'baseline_2026' scenario as the no-shock counterfactual.
    scenarios.insert(0, {
        "scenario_id": "baseline_2026",
        "name": "Baseline 2026",
        "gdp_shock": -0.005,
        "rate_shock_bps": 0.0,
        "sector_shocks": {},
        "_provenance": {"sources": ["hand-anchored"], "note": "no-shock baseline"},
    })

    out = {
        "version": "v0.1.0-calibrated",
        "generated_from": "data/scripts/calibrate/scenarios.py",
        "scenarios": scenarios,
    }
    write_json(out, "historical_scenarios.json")
    return out


if __name__ == "__main__":
    import json
    out = calibrate()
    print(json.dumps(out, indent=2, default=str))
