# tyche-cli

Terminal-first front-end to the Tyche simulation core. Installs the `tyche`
binary into your `cargo` toolchain.

```sh
tyche simulate --portfolio examples/synthetic_portfolio/portfolio.json \
               --scenario stagflation_severe --paths 20000

tyche attest --portfolio examples/synthetic_portfolio/portfolio.json \
             --scenario stagflation_severe --signer-key keys/dev.key

tyche verify --record out/record.json
tyche bench
```

All commands accept `--json` for machine-readable output.
