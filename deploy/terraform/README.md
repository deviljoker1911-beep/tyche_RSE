# Tyche Terraform

Infrastructure-as-Code for the Tyche cloud control plane.

## Layout

```
deploy/terraform/
├── org/                       # AWS Organizations + member-account provisioning
├── modules/
│   ├── network/               # VPC, subnets, NAT, VPC endpoints
│   ├── eks/                   # EKS cluster + node groups + addons
│   ├── data/                  # RDS, ElastiCache, ClickHouse, S3 Object Lock
│   └── security/              # KMS, IAM, Security Hub, GuardDuty, Config
└── envs/
    ├── dev/                   # eu-west-1, smallest viable footprint
    ├── staging/               # eu-west-1, prod-shaped but smaller
    ├── prod-eu-west/          # eu-west-1, prod
    └── prod-eu-central/       # eu-central-1, prod (DR pair)
```

## State backend

S3 + DynamoDB lock table, both in the `mgmt` account. Per-env workspaces
are scoped by key prefix (`tyche/{env}/terraform.tfstate`). Cross-account
plan / apply uses IAM role assumption — no long-lived access keys.

## Reproducibility

- Terraform pinned to `1.10.x` via `.tool-versions`
- AWS provider pinned to `~> 5.83` in every module
- All `random_id` / `random_password` resources keyed off a stable input
  so a re-plan never wants to replace them
- `terraform plan -lock-timeout=10m` in CI

## Multi-account model

| Account | Purpose | IAM identity |
|---|---|---|
| `mgmt` | Org root, billing, IAM Identity Center, log archive | Single bootstrap principal, then SSO only |
| `security` | GuardDuty delegated admin, Security Hub aggregator, audit-log bucket | Read-only access from every member account |
| `dev` | Sandboxes, ephemeral clusters | Per-engineer roles via IAM Identity Center |
| `staging` | Pre-prod, mirrors prod | Limited human access; CI deploys |
| `prod-eu-west` | Production EU-West | No human access in steady state |
| `prod-eu-central` | Production EU-Central (DR) | No human access in steady state |

## Run order

1. `org/` first — creates accounts, sets baselines, configures SCPs
2. Per-env: `network/` → `data/` → `eks/` → application Helm install (out of band)
3. `security/` runs against every account on a daily schedule via GitHub Actions
