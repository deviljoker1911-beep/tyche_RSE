# Runbook — `TycheAttestationAnchorLag`

**Alert**: no Merkle root anchored on chain in over 2 hours.

**Severity**: page.

**Why it pages**: the entire "Proof of Risk" claim breaks down if anchors aren't being published. Customers expect at most one-hour anchor cadence; two hours is the absolute SLA floor.

## First 5 minutes

1. Check the batcher is alive:
   ```
   kubectl -n tyche-prod get pods -l tyche.network/component=batcher
   kubectl -n tyche-prod logs -l tyche.network/component=batcher --tail=200
   ```
2. Check chain RPC reachability:
   ```
   kubectl -n tyche-prod run -it --rm probe --image=curlimages/curl --restart=Never \
     -- curl -s "$ANCHOR_CHAIN_RPC" -X POST -H 'content-type: application/json' \
        -d '{"jsonrpc":"2.0","method":"eth_blockNumber","id":1}'
   ```
3. Check the publisher wallet balance:
   ```
   cast balance 0x<publisher-addr> --rpc-url $ANCHOR_CHAIN_RPC
   ```

## Triage decision tree

### Batcher is down

- Pod CrashLoopBackOff → `kubectl describe pod` — usually means the publisher key secret is missing or malformed.
- Pod is running but logs are silent → leader election may be stuck. Delete the Lease:
  ```
  kubectl -n tyche-prod delete lease tyche-batcher-leader
  ```
- Pod is running and logging "no pending records" → false positive: nothing to anchor. Verify by checking `tyche_pending_records` metric.

### Chain RPC failing

- 503/timeout from the RPC → switch endpoints. Update the secret in AWS Secrets Manager:
  ```
  aws secretsmanager update-secret \
    --secret-id tyche/prod/tyche-batcher-rpc-url \
    --secret-string '{"url":"https://<backup-endpoint>"}'
  kubectl -n tyche-prod delete secret tyche-batcher-rpc-url  # ESO recreates
  ```
- Rate-limited by the primary RPC → the batcher's backoff should handle this. If not, raise an issue.

### Wallet out of gas

- Balance below 0.05 ETH (or equivalent) → top up immediately:
  - Production publisher wallet is the multisig at `0x<...>` (see `docs/contracts/keys.md`).
  - Bridge from cold wallet if you have multisig signers available; otherwise page CFO + CTO.

### Tx stuck

- Pending tx with low gas price → cancel and replace:
  ```
  cast send --rpc-url $ANCHOR_CHAIN_RPC --private-key $PUBLISHER_KEY \
    --nonce <stuck-nonce> --gas-price <2x current> $YOUR_ADDR --value 0
  ```
- Don't bump nonce too aggressively — overlap risks double-publish, which the contract reverts on but burns gas.

## After the incident

- Was the alert delayed by metric scrape gap? The alert fires when `time() - tyche_last_anchor_unix_seconds > 7200`; if the batcher stopped exporting the metric at all, the absent() check should fire too.
- Add chaos-engineering test: simulate the failed dependency and confirm alerts trigger.
- Update SLO compliance: any anchor gap > 1h burns the freshness budget. File it.
