# Runbook — `TycheBatcherDown`

**Alert**: the attestation batcher singleton is not reporting `up == 1` for 5 minutes.

**Severity**: page.

**Why it pages**: every minute the batcher is down is a minute of records piling up in the queue, increasing anchor lag and time-to-publish for customers.

## Sequence

1. `kubectl -n tyche-prod get pod -l tyche.network/component=batcher`
2. If the pod is `Pending`:
   - Check node pressure: `kubectl -n tyche-prod describe pod <pod>` → look at Events.
   - If memory/cpu pressure → bump node group (see [api-latency runbook](./api-latency.md#capacity-bound)).
   - If `volume binding`: probably waiting for an EBS-backed PVC — we don't have one, so this would be an unexpected change.
3. If the pod is `CrashLoopBackOff`:
   - `kubectl logs <pod> --previous` for the last good crash.
   - 99% of the time: a secret is missing or malformed. Check ESO status:
     ```
     kubectl -n tyche-prod get externalsecrets
     kubectl -n tyche-prod describe externalsecret tyche-batcher-publisher-key
     ```
4. If the pod is `Running` but the alert still fires:
   - The pod is up but metric scrape is failing. Check ServiceMonitor:
     ```
     kubectl -n tyche-prod get servicemonitor tyche-batcher
     curl http://<pod-ip>:9090/metrics
     ```

## Failover

The batcher is a **singleton with leader election** via K8s Lease, not a stateful HA setup. To accelerate failover during a node drain:

```
kubectl -n tyche-prod delete lease tyche-batcher-leader
```

This releases the lease; a replacement pod will pick it up within ~10s.

## Capacity backlog after recovery

When the batcher resumes after an outage, it processes the backlog of records in chunks. The metrics to watch during recovery:

- `tyche_pending_records` — should drop steadily toward zero.
- `tyche_anchor_duration_seconds` p95 — may rise briefly because batches are larger than steady state.

Do **not** restart the batcher mid-backlog-flush unless you have to. Each restart costs ~30s of lease-acquire + state-rebuild.

## After the incident

- If the outage was caused by a missing secret, post-mortem the secret-rotation runbook (`docs/runbooks/secret-rotation.md`).
- If the outage was caused by the leader-election Lease being orphaned (we've seen this once during a K8s upgrade), open an issue for an explicit lease-TTL refresh probe.
