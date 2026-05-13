# Tyche Security Policy

**Version**: v0.2 (M1B)
**Last update**: 2026-05

This document is the public-facing contract for how Tyche handles security
issues. It complements [`docs/THREAT_MODEL.md`](THREAT_MODEL.md), which is
the engineering-side analysis.

---

## 1. Scope

| In scope | Out of scope |
|---|---|
| Anything in this repo on `main` | Forks, third-party deployments |
| Released binaries on GHCR / crates.io / npm / PyPI | Customer-side configuration mistakes |
| Anvil + testnet deployments | Mainnet (not deployed yet) |
| Browser verifier bundles signed by us | Compromised CDNs we don't control |
| Cryptographic primitives we ship | Side-channel attacks on underlying CPUs |

---

## 2. What the spike is — and is not

### Is

- A reference implementation of L1 (data graph), L2 (simulation), L3
  (attestation), with skeletons for L4 (federation) and L5 (UX).
- Compiled from audited cryptographic libraries (`curve25519-dalek`,
  `ed25519-dalek`, `sha2`) and a hand-rolled domain-separated SHA-256
  Merkle tree.
- Wired to local Anvil. No mainnet deployment.
- `cargo audit` clean, `cargo deny` (bans/licenses/sources) clean,
  Slither clean (no high/critical findings).
- `pnpm audit --audit-level=high` clean. Remaining moderate advisories
  are dev-tooling only (esbuild/vite dev-server, postcss build).

### Is **not**

- **Not third-party-audited.** Trail of Bits or Zellic audit is the M3
  deliverable. Until that audit lands, do not use Tyche to attest
  production data.
- **Not range-proven.** Federation aggregator is a skeleton — see
  GitHub issue #1.
- **Not threshold-signed.** Single-publisher access control today;
  FROST integration tracked in issue #2.
- **Not GDPR-reviewed.** Adopters must run their own DPIA.
- **Not post-quantum.** Ed25519 + Ristretto255 are pre-quantum. PQ
  migration is on the Y3+ roadmap.

---

## 3. Reporting a vulnerability

### Email
`security@tyche.network`

### PGP (optional but preferred for high-severity reports)
Key ID: `0xTBD-pending-key-ceremony-M2`
Fingerprint: published in `docs/security/pubkey.asc` once the M2 key
ceremony completes. Until then, email plain-text reports are accepted
and routed to a small triage list.

### GitHub Security Advisory (recommended)
Open a [private security advisory](https://github.com/deviljoker1911-beep/tyche_RSE/security/advisories/new)
— this gives you a coordinated-disclosure workflow that we can interact
with directly without anything becoming public.

### Bug bounty
A no-payout listing is live on
[HackerOne](https://hackerone.com/tyche) (link reserved; activation
scheduled in M2). Hall-of-fame credit + swag for valid reports.
Monetary bounties begin once we hit our first revenue milestone — we
don't pretend to have money we don't have.

---

## 4. What to include in a report

1. **Summary** — one paragraph
2. **Reproduction** — minimal steps or PoC
3. **Affected component** — which crate / contract / package
4. **Affected versions** — git SHA or tag if known
5. **Impact** — what an attacker gains
6. **Suggested mitigation** if you have one

Reports that follow this structure are triaged fastest.

---

## 5. Triage SLAs

| Severity | Acknowledge | Initial assessment | Fix landed | Public disclosure |
|---|---|---|---|---|
| **Critical** (RCE, key compromise) | 24 h | 72 h | 7 days | Coordinated, after fix |
| **High** | 48 h | 7 days | 21 days | Coordinated, after fix |
| **Medium** | 7 days | 14 days | 60 days | Coordinated or 90-day default |
| **Low** | 14 days | 30 days | next release | After fix |

All times are business days. Severity is assigned via CVSSv3.1; we will
share the calculation in the response.

---

## 6. Safe-harbour

If you act in **good faith** and follow this policy, we will:

- not pursue or support any legal action against you;
- treat you as if you were authorised under the Computer Fraud and
  Abuse Act (US) / Computer Misuse Act (UK) for the actions necessary
  to find and report the vulnerability;
- credit you publicly if you wish, or keep your involvement private if
  you prefer.

"Good faith" means: no DoS, no data exfiltration beyond what's needed
to prove the issue, no social engineering of our staff, no testing on
other firms' deployments.

---

## 7. Disclosure policy

We follow **coordinated disclosure**. Default disclosure timeline is
**90 days** from initial report, or 7 days after a fix is publicly
available — whichever is shorter. We will not unilaterally extend
beyond 120 days without your agreement.

If a vulnerability is being actively exploited in the wild we may
shorten the timeline to expedite the patch.

---

## 8. Cryptographic primitives

| Primitive | Library | Domain tag |
|-----------------------|--------------------------------|-------------------------------|
| SHA-256 | `sha2` | per-use (see below) |
| Hash commitment | `tyche-crypto::hash_commit` | `tyche/v0.1/hash-commit` |
| Pedersen commitment | `curve25519-dalek` (Ristretto255) | `tyche/v0.1/pedersen-h` |
| Merkle leaf | SHA-256 | `tyche/v0.1/merkle-leaf` |
| Merkle internal node | SHA-256 | `tyche/v0.1/merkle-node` |
| Attestation signature | `ed25519-dalek` | `tyche/v0.1/attestation-record` |
| Object hash | SHA-256 over canonical JSON | `tyche/v0.1/object` |
| Deterministic RNG | `rand_chacha::ChaCha20Rng` | seeded per simulation |
| Cryptographic RNG | `rand::rngs::OsRng` | OS entropy |

Domain separation is mandatory on every hash use. Tag changes are
breaking and require a `model_version` bump on records.

---

## 9. Reproducibility & supply-chain

We commit to:

- **Reproducible builds** — same source + same toolchain = same binary
  hash. Rust pinned via `rust-toolchain.toml`; Foundry pinned in CI.
- **SBOM** with every release (CycloneDX format, generated by `syft`).
- **Signed releases** — every binary and container signed with
  [cosign](https://docs.sigstore.dev/cosign/overview/) using GitHub
  OIDC identity.
- **SLSA L2 provenance** — every release artifact has machine-checkable
  provenance attestation. SLSA L3 is M4.
- **Daily advisory scan** — `cargo audit` runs against the latest
  RustSec database every 24h via `.github/workflows/security.yml`.

To verify a released binary:

```sh
# Verify signature
cosign verify --certificate-identity-regexp '.*' \
              --certificate-oidc-issuer https://token.actions.githubusercontent.com \
              ghcr.io/deviljoker1911-beep/tyche@<digest>

# Inspect SBOM
cosign download sbom ghcr.io/deviljoker1911-beep/tyche@<digest> | jq .
```

---

## 10. Hardening features in this release

- All Rust crates declare `#![forbid(unsafe_code)]`.
- No `unwrap()` in non-test code (M1A audit).
- `cargo clippy` runs at `pedantic + nursery` level.
- All hashes domain-separated.
- All randomness sources documented and partitioned (deterministic vs
  OS entropy).
- WAF + rate limit + circuit breaker at API edge (M1H).
- Strict CSP + HSTS + SRI on the web app (M2).

---

## 11. Out-of-scope assertions

We do not currently make claims about:

- **Quantum resistance** — see roadmap.
- **Defence against malicious firm operators with HSM access** — see
  threat model section 3, T3.
- **Side-channel resistance** of the simulator (timing variance can
  leak loan count; not values).
- **Privacy of attestation timestamps** on chain — they're public by
  design.

---

## 12. Version history

| Version | Date | Changes |
|---|---|---|
| 0.1 | 2026-05 | Initial spike-phase document |
| 0.2 | 2026-05 | M1B baseline: audit pipeline, threat model, SLA table, safe-harbour, supply-chain commitments |

Updates to this document are tracked in `git log -- docs/SECURITY.md`.
