# tyche-py

Python bindings for the Tyche simulation core via PyO3.

Build with `maturin develop --release` from this directory; the resulting
extension module exposes `tyche_sim.simulate(portfolio_json, scenario_json,
n_paths)` and is intended for use from research notebooks under `notebooks/`.
