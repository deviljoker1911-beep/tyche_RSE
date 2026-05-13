# Tyche — Threat Model

**Version**: v0.1 (M1B baseline)
**Last review**: 2026-05
**Methodology**: STRIDE per layer, with attacker tiers per asset.

> This document is a living artifact. Any change that alters a trust
> boundary, introduces a new data sink, or changes a cryptographic primitive
> **must** be paired with a PR that updates this file. Reviewers may block
> merges that do not.

---

## 1. Assets

| Asset | Owner | Sensitivity | Why an attacker wants it |
|---|---|---|---|
| **Portfolio data** (loans, covenants, leverage) | Credit fund | **Critical** — never leaves the firm | Front-running, market intelligence, regulatory leverage |
| **Simulation outputs** (risk metrics) | Credit fund | High — fund's confidential view of own risk | Insider trading, LP renegotiation |
| **Ed25519 signing keys** | Firm-operator | **Critical** | Forge attestations attributed to the firm |
| **Pedersen blinding factors** | Firm-operator | High | Open a hash commitment after the fact |
| **Attestation records** (signed) | Firm + LP + Regulator | Public when published | Modify history → loss of audit trail; replay → double-attest |
| **Merkle roots on chain** | Public | Public | Modify or delete → loss of integrity proof |
| **Federation aggregate** | Network | Sensitive — sum of all firms | Reverse-engineer individual contributions |
| **HSM / KMS keys** | Tyche-platform | **Critical** | Compromise every customer simultaneously |
| **CI/CD pipeline** | Tyche-platform | **Critical** | Supply-chain attack: inject backdoor into production binary |
| **Customer credentials** (SSO sessions, API keys) | Tyche-platform | High | Lateral movement into customer environments |

---

## 2. Attacker tiers

| Tier | Profile | Capability | Realistic targets |
|---|---|---|---|
| **T1 — External web** | Anonymous internet | HTTP requests, public chain reads | Public API endpoints, chain data, NPM/Cargo registry attacks |
| **T2 — Authenticated user** | Logged-in firm user (insider) | Anything T1 + their session's API surface | Their own firm's data, attempting privilege escalation |
| **T3 — Compromised firm-operator** | Stolen credentials of a firm admin | Anything T2 + secret key access | Sign forged attestations, replay records, exfiltrate keys |
| **T4 — Compromised Tyche-platform engineer** | Insider with prod access | Anything T3 across all customers + infra plane | Multi-tenant compromise, supply-chain attack |
| **T5 — Nation-state / forensics** | Sophisticated, well-funded, patient | Side-channel, cryptanalysis, physical access to HSMs | Long-horizon forensics, breaking older deprecated primitives |

The spike design is hardened against T1–T3. T4 mitigations land in M2 (HSM, dual-key control, audit-log immutability). T5 is out of scope for the open-source reference implementation; defenders should assume audit-only posture.

---

## 3. Trust boundaries

```
┌───────────────────────────────────────────────────────────────────────┐
│  Firm's network (on-prem or VPC)                                      │
│  ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐   │
│  │ Risk analyst's  │    │ Tyche on-prem    │    │ HSM / KMS       │   │
│  │ browser         │◄──►│ agent / CLI      │◄──►│ (signing key)   │   │
│  └─────────────────┘    └─────┬────────────┘    └─────────────────┘   │
│                               │                                       │
│                  mTLS         │  outbound only                        │
└───────────────────────────────┼───────────────────────────────────────┘
                                │
┌───────────────────────────────▼───────────────────────────────────────┐
│  Tyche cloud (multi-tenant)                                           │
│  ┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐   │
│  │ API gateway     │◄──►│ Attestation      │◄──►│ Federation      │   │
│  │ (WAF + rate)    │    │ batcher          │    │ aggregator      │   │
│  └─────────────────┘    └─────┬────────────┘    └─────────────────┘   │
│                               │                                       │
└───────────────────────────────┼───────────────────────────────────────┘
                                │
                  ┌─────────────▼─────────────┐
                  │ Public chain (Ethereum)   │ ◄── LP / regulator / auditor
                  └───────────────────────────┘
```

Three trust boundaries:
1. **Firm ↔ Tyche cloud** — only hashes, commitments, signed records cross; never plaintext portfolios.
2. **Tyche cloud ↔ chain** — only Merkle roots cross; chain is a public bulletin board.
3. **Chain ↔ verifiers (LP / regulator)** — public; no inbound trust.

---

## 4. STRIDE — per layer

### 4.1 — Simulation core (L2 — `tyche-sim`)

| STRIDE | Threat | Mitigation | Status |
|---|---|---|---|
| **S**poofing | A malicious binary claims to be `tyche-sim` and emits forged outputs | Reproducible builds + signed releases (cosign) + SLSA L2 provenance | M1B in flight; full L3 in M3 |
| **T**ampering | Non-determinism in float math gives different output for same input | `serde_json::float_roundtrip` + canonical JSON + ChaCha20 RNG (M1A); fixed-point math option deferred to M5 | Mitigated for current scope |
| **R**epudiation | Firm denies running the simulation that produced a record | Ed25519 signature over canonical input hash + result hash binds firm to record | Mitigated |
| **I**nfo disclosure | Side channel via timing variance leaks portfolio composition | Constant-time crypto where applicable (`subtle::ConstantTimeEq`); simulator timing is *not* constant-time | Accepted risk — simulation timing leaks loan count but not values |
| **D**oS | Adversarial portfolio shape causes exponential blow-up | `chunk_size` bounds memory; validation rejects empty / NaN inputs; per-tenant rate limit | Mitigated at validation layer |
| **E**levation | Sim core escalates to host shell | `#![forbid(unsafe_code)]` on every crate; `unwrap()` audit done in M1A | Mitigated |

### 4.2 — Crypto primitives (`tyche-crypto`)

| STRIDE | Threat | Mitigation |
|---|---|---|
| **S** | Forged Pedersen commitment | Only audited libs: `curve25519-dalek`, `sha2`, `ed25519-dalek`. No hand-rolled field math |
| **T** | Merkle proof tampering | Domain-separated hashes (`DOMAIN_MERKLE_LEAF` vs `DOMAIN_MERKLE_NODE`) prevent second-preimage |
| **R** | Verifier denies seeing a valid proof | Verification is deterministic and pure — anyone can re-verify |
| **I** | Side channel on signature verification | `subtle::ConstantTimeEq` for hash commitment equality |
| **D** | Pathological Merkle input | Tree construction is `O(n)` with bounded memory; empty case has a well-defined root |
| **E** | Buffer overflow / memory unsafety | `#![forbid(unsafe_code)]` |

### 4.3 — Attestation (L3 — `tyche-attest` + Solidity)

| STRIDE | Threat | Mitigation |
|---|---|---|
| **S** | Attacker publishes Merkle root claiming to be from firm X | On-chain `AttestationRegistry` uses OZ `AccessControl`; only `PUBLISHER_ROLE` can publish. Off-chain records carry Ed25519 sig binding to firm pubkey |
| **T** | Modify a record after publication | Record content-hash committed in Merkle leaf; any change invalidates inclusion proof |
| **R** | Firm publishes then claims "we didn't" | Ed25519 sig + on-chain transaction sender = bound to firm wallet |
| **I** | LP can read other firms' records from chain | Chain stores only roots, never leaves. Records are off-chain and access-controlled |
| **D** | Spam `publishRoot` calls | `PUBLISHER_ROLE` gate + per-tenant rate-limit at API gateway |
| **E** | Bug in `verifyInclusion` accepts forged proofs | 100% test + branch coverage on `AttestationRegistry.sol`; uses audited OZ `MerkleProof` library |

### 4.4 — Federation (L4 — `tyche-fed`, skeleton)

| STRIDE | Threat | Mitigation | Status |
|---|---|---|---|
| **S** | Firm submits contribution claiming to be another firm | `firm_id_hash` is the identity; duplicate-detection in `aggregate()` | Mitigated |
| **T** | Firm publishes commitment to negative or absurd value | Bulletproofs+ range proof | **Not yet** — tracked in issue #1, scheduled M3 |
| **R** | Firm denies submitting an aggregated contribution | Submissions signed on chain (`submit()`); chain tx sender = firm wallet | Mitigated |
| **I** | Reverse-engineer individual contributions from aggregate | Pedersen commitments are perfectly hiding; aggregate reveals only the sum | Mitigated by construction |
| **D** | Submission flood across rounds | Per-round deadline + `PUBLISHER_ROLE` gate | Mitigated |
| **E** | FROST threshold signer compromise | FROST threshold signing | **Not yet** — issue #2, M3 |

### 4.5 — Web app (`apps/web` + WASM core)

| STRIDE | Threat | Mitigation |
|---|---|---|
| **S** | Phishing site impersonates Tyche | HSTS + CAA records + EV cert; canonical domain published in `SECURITY.md` |
| **T** | Tampered WASM bundle served to user | Subresource integrity hash; signed bundle (cosign) verified at build time |
| **R** | User denies attesting via the browser | Signing happens server-side under firm-owned key; browser is presentation only |
| **I** | XSS exfiltrates portfolio data from browser memory | Strict CSP; no `dangerouslySetInnerHTML`; React 19 escaping; CodeQL in CI |
| **D** | Browser-side DoS via malformed scenario | Zod validation + bounded chunk size; WASM panic hook surfaces error gracefully |
| **E** | Wallet-prompt injection from compromised wagmi dep | Pinned wagmi + viem versions; `pnpm audit --audit-level=high` gates CI |

### 4.6 — CI/CD pipeline

| STRIDE | Threat | Mitigation |
|---|---|---|
| **S** | Malicious commit signed as a maintainer | Signed commits (sigstore/cosign); branch protection on `main` |
| **T** | Build artifact differs from source | Reproducible builds + SLSA L2 provenance + cosign signatures (in M1B CI scaffold) |
| **R** | Release published without attribution | All releases via `release-please` automation, signed by GH OIDC identity |
| **I** | Secrets exposed in CI logs | gitleaks scan in CI; secret values masked via `::add-mask::` |
| **D** | Workflow DoS via huge PRs | Concurrency limits in workflow YAML |
| **E** | Workflow gains write to prod | Minimal `permissions:` on each job; OIDC instead of long-lived AWS keys (M1D) |

---

## 5. Cryptographic primitives — provenance

| Primitive | Source | Why this choice | Rotation policy |
|---|---|---|---|
| SHA-256 | `sha2` crate (RustCrypto) | NIST-standard, hardware-accelerated, no known weakness | Replace if a structural attack lands; migration plan = bump `HASH_DOMAIN` |
| BLAKE3 | `blake3` crate | Faster hash for hot paths; reserved for future use | — |
| Ed25519 | `ed25519-dalek` v2 | Standard, audited, fast verify, deterministic signatures | Per-firm key rotation policy at customer onboarding; HSM-managed in prod |
| Ristretto255 | `curve25519-dalek` v4 | Prime-order group, no cofactor footguns, Pedersen-friendly | — |
| Merkle (binary SHA-256, duplicated odd leaf) | `tyche-crypto` (hand-rolled with audited primitives) | Simple, matches OZ Solidity verifier behaviour | — |
| RNG (deterministic) | `rand_chacha::ChaCha20Rng` | Stream cipher = forward-secure; deterministic for reproducibility | Re-seed per simulation |
| RNG (key material) | OS RNG via `rand::rngs::OsRng` | OS-managed entropy source | — |

Every hash usage has an **explicit domain tag** (e.g. `b"tyche/v0.1/merkle-leaf"`) to prevent cross-protocol confusion. See `crates/tyche-crypto/src/lib.rs`.

---

## 6. What this threat model does **not** cover

- **Quantum adversaries.** Ed25519 and Ristretto255 are pre-quantum. Post-quantum migration is on the Y3+ roadmap.
- **Long-term key custody.** Customers control their HSMs; Tyche's recommendations are documentary.
- **Hardware-level attacks** (cold boot, Rowhammer, fault injection). Out of scope; assume CSP-level physical security.
- **Social engineering** against firm staff. Out of scope; customers' MFA + SOC.
- **Regulatory compliance** as a security property. Compliance is documented in `docs/COMPLIANCE.md` (Phase 2).

---

## 7. Review cadence

- **Per PR**: reviewer must confirm no trust-boundary change escapes notice.
- **Quarterly**: walk the table, re-rank status, file issues for newly identified gaps.
- **Per audit**: append auditor findings as an annex.
- **Per crypto-primitive upgrade**: section 5 must be updated in the same PR.

---

## 8. Mitigation roadmap snapshot

| Risk | Mitigation | Milestone |
|---|---|---|
| Federation range-proof gap | Bulletproofs+ integration | M3 (issue #1) |
| Federation threshold-sig gap | FROST-Ed25519 | M3 (issue #2) |
| Solidity range-proof verifier | On-chain Bulletproofs+ | M3 (issue #3) |
| Side-channel timing on simulator | Constant-time simulation (or accepted) | Discussion required, M5 |
| HSM integration | AWS CloudHSM + YubiHSM2 | M2 |
| Multi-tenant data isolation | Cell-based architecture | M5 |
| Post-quantum migration | Hybrid Ed25519 + Dilithium | Y3+ |
