# Observability

Stack reference and on-call paths for Tyche.

## What runs where

| Layer | Tool | Purpose |
|---|---|---|
| Traces | OpenTelemetry Collector → Tempo | Distributed request tracing across `tyche-api` → simulator → attestation |
| Logs | Tracing-subscriber JSON → OTEL Collector → Loki | Structured logs; PII-scrubbed before egress |
| Metrics | Prometheus scrape + remote-write | Golden signals + Tyche-specific gauges |
| Dashboards | Grafana | Six dashboards under `observability/grafana/dashboards/` |
| Alerting | Prometheus rules in Helm | Pages on the 4 page-class alerts |
| Errors | Sentry (optional, opt-in) | Stack-traced application errors |
| Cost | OpenCost / KubeCost | Per-namespace and per-deployment $$ |

## The six golden dashboards

| # | UID | Audience |
|---|---|---|
| 01 | `tyche-api-golden` | Engineer on call — request rate / errors / latency / replicas |
| 02 | `tyche-attestation-pipeline` | Anyone — how the attestation pipeline is performing |
| 03 | `tyche-simulation-perf` | Quants + engineering — throughput, paths/sec, CPU |
| 04 | `tyche-slo-budget` | Leadership — 30-day error-budget burn |
| 05 | `tyche-chain-anchoring` | Engineer on call — chain side health |
| 06 | `tyche-infra-cost` | Finance + engineering — $/attestation, headroom |

## Alerting

Four `severity: page` alerts (defined in `deploy/helm/tyche-platform/templates/common/monitoring.yaml`):

1. `TycheApiHighErrorRate` → [`docs/runbooks/api-error-rate.md`](../docs/runbooks/api-error-rate.md)
2. `TycheApiHighLatency` → [`docs/runbooks/api-latency.md`](../docs/runbooks/api-latency.md)
3. `TycheAttestationAnchorLag` → [`docs/runbooks/anchor-lag.md`](../docs/runbooks/anchor-lag.md)
4. `TycheBatcherDown` → [`docs/runbooks/batcher-down.md`](../docs/runbooks/batcher-down.md)

Every alert carries the `runbook_url` annotation so on-call sees the link in PagerDuty directly.

## SLOs

| Service | Indicator | Objective | Window |
|---|---|---|---|
| `tyche-api` | Availability (1 − 5xx-rate) | 99.9% | 30d |
| `tyche-api` | Latency p99 | < 5s for 99% of requests | 30d |
| Attestation anchoring | Max anchor age | < 1h | 30d |
| Verification | Inclusion-proof success rate | > 99.99% | 30d |

Error budget burn is plotted on dashboard #4.

## Log format

In production, `TYCHE_LOG_FORMAT=json` flips the CLI/API to structured JSON:

```json
{"timestamp":"2026-05-13T11:42:01.123Z","level":"INFO","target":"tyche_api::handlers",
 "fields":{"message":"simulation completed","sim_id":"sim_01HZK...","n_paths":20000,
           "duration_ms":143.2,"tyche_tenant":"hashed_abc..."}}
```

PII fields are hashed before they leave the pod (see `observability/otel/collector.yaml`,
`processors.attributes/scrub`).

## Local development

`docker-compose.yml` (M1C) brings up Prometheus + Grafana on `localhost:9090` and
`:3001`. Dashboards auto-provision from this directory.
