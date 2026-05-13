# tyche-api

HTTP front-end to the Tyche simulation + attestation core.

Routes (M1C scaffold):

| Method | Path | Purpose |
|---|---|---|
| `GET`  | `/healthz`           | Liveness probe — always 200 if the process is up |
| `GET`  | `/readyz`            | Readiness probe — returns 200 only when dependent services are reachable |
| `GET`  | `/version`           | Build metadata (commit SHA, model_version, build time) |
| `POST` | `/v1/simulate`       | Run a Monte Carlo simulation and return `RiskMetrics` |
| `POST` | `/v1/attest`         | Sign and persist an attestation record |
| `POST` | `/v1/verify`         | Verify a record's signature |

In M1C the server is **stateless** — no DB / Redis yet. M1G adds the
data layer; M1H adds rate limiting and circuit breakers.

## Run locally

```sh
cargo run -p tyche-api
# or
docker run --rm -p 8080:8080 ghcr.io/deviljoker1911-beep/tyche-api:latest
```

Bind address is configurable via `TYCHE_API_ADDR` (default `0.0.0.0:8080`).
