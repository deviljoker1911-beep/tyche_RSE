# Tyche contracts

Foundry workspace for the on-chain Tyche layer.

## Setup

```sh
cd contracts
# Pinned versions — change here and in .github/workflows/solidity.yml together.
forge install --no-git --shallow \
  foundry-rs/forge-std@v1.9.4 \
  OpenZeppelin/openzeppelin-contracts@v5.0.2 \
  Vectorized/solady@v0.1.16
forge build
forge test -vvv
forge coverage --report summary
```

`contracts/lib/` is gitignored — every clone needs `forge install`. CI does
this automatically via `.github/workflows/solidity.yml`.

## Deploy locally

```sh
anvil &
forge script script/Deploy.s.sol:Deploy --rpc-url http://localhost:8545 --broadcast
```

The deploy script prints the registry and aggregator addresses; the web app
reads them from `apps/web/lib/contracts.json` (populated by `scripts/dev.sh`).
