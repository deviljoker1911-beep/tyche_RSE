# Tyche research notebooks

Quant research and reference implementations live here. These are **not**
production code — they exist to validate the Rust simulator's outputs against
independent Python implementations and to explore parameter sensitivity.

Recommended setup:

```sh
# from repo root
nix develop                         # if using the Nix flake
maturin develop --release \
  --manifest-path crates/tyche-py/Cargo.toml
jupyter lab notebooks/
```

The Python module `tyche_sim` is built from `crates/tyche-py` via maturin
and exposes a single function: `tyche_sim.simulate(portfolio_json,
scenario_json, n_paths, seed=None)`.

## Notebooks

- `sanity.ipynb` — a side-by-side comparison of the Rust simulator and a
  pure-Python reference implementation. Asserts that both produce the same
  expected loss to within Monte Carlo error.
- `factor_sensitivity.ipynb` *(planned)* — VaR / ES under sweeps of
  `sector_correlation` and `market_correlation`.
