"""Recovery floors per seniority, anchored against public studies.

This module does not call any numerical optimiser — recovery
distributions are not derivable from the macro datasets we have. Instead
it codifies the published priors from cited sources, so the simulator's
``Seniority::base_recovery_rate`` can be defended in a methodology
document.

Sources:

- Moody's "Ultimate Recovery Database" annual summaries — public
  aggregates by seniority class (2010-2023).
- S&P Global Ratings "Recovery Study" annuals — public summary tables.
- ECB / EBA "NPL workout outcomes" reports — EU-specific recovery
  rates for senior secured corporate exposures (typically lower than
  US-sourced Moody's figures due to slower European resolution).

The numbers below are conservatively at the lower end of the published
ranges; that's appropriate for a stress-testing tool. If the
methodology section needs higher numbers, change them here and re-run
``calibrate``.
"""

from __future__ import annotations

import json

from ._paths import write_json


# Aligned with Tyche's `Seniority` enum. Values are 1y point estimates
# of ultimate recovery rate as a fraction of par.
RECOVERY_FLOORS: dict[str, dict] = {
    "senior_secured": {
        "central": 0.65,
        "lower_5th": 0.45,
        "upper_95th": 0.80,
        "source": "Moody's URD 2010-2023 senior secured; EBA workout reports",
    },
    "second_lien": {
        "central": 0.45,
        "lower_5th": 0.20,
        "upper_95th": 0.65,
        "source": "Moody's URD 2nd-lien tranches; S&P EU recovery study",
    },
    "senior_unsecured": {
        "central": 0.40,
        "lower_5th": 0.15,
        "upper_95th": 0.60,
        "source": "Moody's URD senior unsecured corp; S&P recovery report",
    },
    "subordinated": {
        "central": 0.25,
        "lower_5th": 0.05,
        "upper_95th": 0.50,
        "source": "Moody's URD subordinated debt",
    },
    "mezzanine": {
        "central": 0.15,
        "lower_5th": 0.00,
        "upper_95th": 0.40,
        "source": "S&P / Preqin mezz fund return commentary (illustrative)",
    },
    "equity": {
        "central": 0.05,
        "lower_5th": 0.00,
        "upper_95th": 0.20,
        "source": "Moody's URD equity-like tranches",
    },
}


def calibrate() -> dict:
    out = {
        "version": "v0.1.0-calibrated",
        "generated_from": "data/scripts/calibrate/recovery.py",
        "spike_anchor_method": "hand-picked priors from ad-hoc reading",
        "calibrated_anchor_method": "consensus mid of published aggregates",
        "recovery_floors": RECOVERY_FLOORS,
        "_notes": [
            "We deliberately use the CENTRAL value as the simulator's",
            "single point estimate. Phase 2 will replace this with a",
            "Beta(alpha, beta) draw per defaulted loan to capture",
            "stochastic recovery — at which point the lower_5th /",
            "upper_95th fields become the distribution parameters.",
            "",
            "EU mid-market workouts are slower than US ones. The",
            "central numbers above are biased to the EU side of the",
            "published US/EU range (typically 5-10 pts lower than US",
            "senior-secured studies).",
        ],
    }
    write_json(out, "recovery_floors.json")
    return out


if __name__ == "__main__":
    out = calibrate()
    print(json.dumps(out, indent=2, default=str))
