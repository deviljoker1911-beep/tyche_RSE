"""Path helpers shared across calibration modules."""

from pathlib import Path

_THIS = Path(__file__).resolve()
ROOT = _THIS.parents[3]                # tyche repo root
RAW = ROOT / "data" / "raw"
PROC = ROOT / "data" / "processed"
OUT = ROOT / "data" / "calibrated"
OUT.mkdir(parents=True, exist_ok=True)


def write_json(payload: dict, name: str) -> Path:
    """Write a JSON artifact under ``data/calibrated/`` and return the path."""
    import json
    path = OUT / name
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, default=str))
    return path
