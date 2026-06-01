# tyche-api

HTTP front-end to the Tyche simulation + attestation core.

## Routes

| Method | Path | Purpose |
|---|---|---|
| `GET`  | `/healthz`     | Liveness — 200 while the event loop is alive |
| `GET`  | `/readyz`      | Readiness — 200 when able to serve (probes deps in M2+) |
| `GET`  | `/version`     | Build metadata (commit SHA, model version, build time) |
| `GET`  | `/metrics`     | Prometheus exposition (scraped by the cluster `ServiceMonitor`) |
| `POST` | `/v1/simulate` | Run a Monte Carlo simulation → `RiskMetrics` |
| `POST` | `/v1/attest`   | Build a signed attestation record |
| `POST` | `/v1/verify`   | Verify a record's signature |

## Resilience (M1H)

Every request passes through this layer onion (outer → inner):

```
SetRequestId → PropagateRequestId → Trace → track_http_metrics
  → resilience( timeout 30s, load-shed, concurrency-limit 256 )
  → rate-limit (per-tenant token bucket)
  → body-limit (1 MiB)
  → handler
```

| Failure | Status | Header |
|---|---|---|
| Body > 1 MiB | `413` | — |
| Per-tenant rate exceeded | `429` | `Retry-After` |
| Concurrency limit hit (load shed) | `503` | `Retry-After` |
| Request > 30 s | `504` | — |
| Bad input / validation | `400` | — |

Tunable via env: `TYCHE_TIMEOUT_SECS`, `TYCHE_MAX_INFLIGHT`,
`TYCHE_RATE_CAPACITY`, `TYCHE_RATE_PER_SEC`.

> The rate limiter is an **in-memory, single-process** stand-in for the spike.
> Production (M2) swaps in a Redis-backed token bucket so the limit is enforced
> across every replica. The swap is a one-file change behind
> `rate_limit::RateLimiter::check`.

## Metrics (M1G)

Emits the series the Grafana dashboards select on: `http_requests_total`,
`http_request_duration_seconds`, `tyche_simulations_total`,
`tyche_simulation_duration_seconds`, `tyche_simulation_n_paths` / `n_loans`,
`tyche_simulation_paths_total`, `tyche_attestations_signed_total`,
`tyche_verifications_total`, `tyche_verification_failures_total`. Two global
labels (`tyche_component=api`, `tyche_env`) are attached by the recorder.

## Tenancy

The rate-limit bucket key comes from the `X-Tyche-Tenant` header. In production
the authenticating gateway sets this from the verified SSO identity and
overwrites any inbound value; in the spike it is trusted as-is.

## Run locally

```sh
cargo run -p tyche-api
# custom bind + verbose logs:
TYCHE_API_ADDR=127.0.0.1:9090 RUST_LOG=info,tyche=debug cargo run -p tyche-api
# container:
docker run --rm -p 8080:8080 ghcr.io/deviljoker1911-beep/tyche-api:latest
```

The binary always logs structured JSON (it runs as a service). The CLI
(`tyche-cli`) is the human-facing surface.
