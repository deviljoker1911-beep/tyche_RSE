"""Tyche calibration pipeline.

Each submodule turns a public dataset into a JSON artifact under
``data/calibrated/`` that the simulator (or its synthetic-portfolio
generator) can consume. The artifacts are deliberately small and
human-readable so the calibration choices are auditable.

See ``data/scripts/calibrate/README.md`` for the dataset → parameter
crosswalk.
"""
