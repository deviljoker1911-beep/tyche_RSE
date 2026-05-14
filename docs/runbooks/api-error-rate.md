# Runbook — `TycheApiHighErrorRate`

**Alert**: 5xx rate from the API > 0.5% over a 10-minute window.

**Severity**: page.

**Why it pages**: a sustained 0.5% 5xx rate burns the monthly error budget (99.9% target) in less than 12 hours. Customers calling attest/verify will see real failures.

## First 5 minutes

1. **Confirm the alert hasn't auto-resolved.** Look at the [API Golden dashboard](https://grafana.tyche.network/d/tyche-api-golden) over the last 15 min. If the curve is dropping, monitor for 10 min and stand down.
2. **Identify the failure surface.** Group the 5xx rate by `route` in the dashboard. Three modes:
   - **Concentrated on a single route**: probably a recent deploy bug.
   - **Concentrated on a single tenant**: probably input-shape pathological. Check the tenant's recent API audit log.
   - **Uniform across routes**: probably a downstream dependency (Postgres / Redis / chain RPC).

## Triage decision tree

### Mode 1 — single-route spike

```
kubectl -n tyche-prod logs -l tyche.network/component=api --tail=200 \
  | jq 'select(.level=="ERROR" or .level=="WARN")'
```

- If the errors mention `panic` or a backtrace → roll back the latest release.
  - `kubectl -n argocd argocd app rollback tyche-prod` (use the previous green sync)
- If the errors mention input validation → check the deploy diff:
  - `git log --oneline -10 main` and identify the suspect commit.

### Mode 2 — single-tenant spike

- Page the tenant via the on-call shared channel; their portfolio submission may be malformed.
- Apply tenant-specific rate-limit if the load is also high:
  ```
  redis-cli -h <prod-redis> SET tyche:rate:tenant:<id>:limit 10
  ```
- If you suspect abuse, escalate to security on-call.

### Mode 3 — uniform spike (dependency)

Walk the dependency list in this order and check each:

| Dependency | Check |
|---|---|
| Postgres | `tyche_db_pool_active` ≥ `tyche_db_pool_max - 1` for >2m → connection exhaustion |
| Redis    | `redis_connected_clients` saturated, `redis_blocked_clients > 0` |
| Chain RPC | `tyche_chain_rpc_duration_seconds` p95 > 5s |
| OTLP collector | `otelcol_exporter_send_failed_metric_points` > 0 (logs/traces dropping, not user impact) |

If Postgres: scale up via Terraform (`serverlessv2_scaling_configuration.max_capacity`) and reapply.
If Redis: failover to replica via ElastiCache console.
If Chain RPC: switch RPC endpoint by patching the `tyche-batcher-rpc-url` secret in AWS Secrets Manager. ESO picks it up within 1h; force-refresh by deleting the K8s Secret.

## After the incident

1. Open an incident report from the [postmortem template](../postmortems/_template.md).
2. Add the actual mitigation step that worked to this runbook.
3. If we hit Mode 1 (deploy bug), open a "missing guardrail" issue — a regression test or a canary check that would have caught it.

## Related

- SLO doc: `docs/slo/api.md`
- Dashboard: [Tyche — API Golden Signals](https://grafana.tyche.network/d/tyche-api-golden)
- Alert source: `deploy/helm/tyche-platform/templates/common/monitoring.yaml`
