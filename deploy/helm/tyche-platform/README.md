# tyche-platform Helm chart

Deploys the Tyche API, web dashboard, and attestation batcher to a
Kubernetes cluster with the security baseline a tier-1 enterprise
customer expects out of the box.

## What it installs

| Resource | Purpose |
|---|---|
| `Deployment / api` | Rust axum service. HPA 3–20 replicas, PDB minAvailable 2. |
| `Deployment / web` | Next.js dashboard. HPA 2–10. |
| `Deployment / batcher` | Singleton Merkle-anchor batcher. Recreate strategy, leader Lease via RBAC role. |
| `ServiceMonitor` + `PrometheusRule` | Auto-discovered by kube-prometheus-stack. Includes four golden alerts (5xx rate, p99 latency, anchor lag, batcher down). |
| `NetworkPolicy` × 3 | Default-deny baseline + explicit allow for gateway, observability and required egress. |
| `ResourceQuota` + `LimitRange` | Per-namespace CPU/memory/pod ceilings. |
| `ServiceAccount` + `Role` + `RoleBinding` | Minimal RBAC: only the Lease verbs needed for leader election. |
| `SecretStore` + `ExternalSecret` × 5 | ESO pulls signing keys, DB URL, Redis URL, RPC URL from AWS Secrets Manager. **No secret values in git.** |

## Pod Security baseline

The chart targets the **`restricted`** Pod Security Standard:

- Non-root (UID 65532), no privilege escalation, all caps dropped
- Read-only root filesystem with explicit tmpfs for writable paths
- `seccompProfile: RuntimeDefault` on every container
- Automatic token mount kept (needed for leader-election RBAC)

Namespaces created by the chart are labelled `pod-security.kubernetes.io/enforce=restricted` so that any future workload that forgets a SecurityContext is rejected at admission.

## Promotion model

- `main` branch → ArgoCD auto-syncs to `tyche-dev` and `tyche-staging`
- Tagged release → manual ArgoCD sync to `tyche-prod` after CODEOWNERS approval

See [`deploy/argocd/applicationset.yaml`](../../argocd/applicationset.yaml).

## Quick start (kind / minikube)

```sh
helm template tyche . \
  --namespace tyche-dev \
  --set global.environment=dev \
  --set global.createNamespace=true | \
  kubectl apply --server-side -f -
```

For production, use ArgoCD — never `helm install` from a developer laptop.
