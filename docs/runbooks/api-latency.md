# Runbook — `TycheApiHighLatency`

**Alert**: API p99 latency > 5s over 15 min.

**Severity**: page.

**Why it pages**: a fund's risk officer triggers an interactive simulation from the web app. Beyond ~5s p99 the workflow breaks down — they hit refresh, retry, and double-pay.

## Symptoms tree

- **Latency rises but error rate is flat** → the system is overloaded but not broken. Capacity issue.
- **Latency rises with error rate** → also overloaded but tipping into failure. Same fix, more urgent.
- **Latency rises only on `/v1/simulate`** → the simulation itself is slow. Look at portfolio size in the offending tenant.

## First 5 minutes

1. Check the [Simulation Performance dashboard](https://grafana.tyche.network/d/tyche-simulation-perf). What's `avg paths/sim` and `paths/sec throughput`?
2. Check current replica count: `kubectl -n tyche-prod get deploy tyche-api -o jsonpath='{.status.replicas}'`. If at HPA max (50) and CPU >90%, you're capacity bound.
3. Check the `Pending records in queue` panel. If climbing, the batcher is the bottleneck, not the API.

## Fixes by symptom

### Capacity bound (CPU saturating, replicas at max)

- **Short term**: bump HPA max:
  ```
  kubectl -n tyche-prod patch hpa tyche-api -p '{"spec":{"maxReplicas":100}}'
  ```
- **Short term 2**: if the node pool is also at max, scale the EKS node group:
  ```
  aws eks update-nodegroup-config --cluster-name tyche-eks --nodegroup-name tyche-default \
    --scaling-config minSize=6,desiredSize=20,maxSize=30
  ```
- **Medium term**: file an issue to raise the steady-state HPA max via Helm values.

### Single tenant submitting huge portfolios

- Identify the tenant: filter the dashboard by `tyche_tenant`.
- Check submission size: `tyche_simulation_n_loans` per tenant.
- If size > 10k loans, talk to the customer about chunking the run (the simulator scales linearly past ~5k loans on default chunk_size).

### Simulation itself is slow (every request)

- Check for a recent change to `crates/tyche-sim/` — the benchmark in CI should have caught it but may have been waived.
- Run `cargo bench -p tyche-sim` on a comparable host; compare against the snapshot in CI artifacts.

## Backpressure

If everything is healthy but the upstream is just oversubscribed, enable load-shed:

```
kubectl -n tyche-prod set env deploy/tyche-api TYCHE_LOAD_SHED_ENABLED=1
```

The API will start returning 503 to ~10% of requests with a Retry-After header. Worse for users than 200 + slow, but bounds the damage.

## After the incident

- Was HPA scaling correctly? If `desiredReplicas` didn't track CPU, the metric collection is broken.
- Did node-autoscaler keep up? If pods stayed Pending for >2 min, Karpenter / cluster-autoscaler needs tuning.
