# Tyche contracts

Foundry workspace for the on-chain Tyche layer.

## Setup

```sh
cd contracts
forge install foundry-rs/forge-std OpenZeppelin/openzeppelin-contracts@release-v5.0 Vectorized/solady
forge build
forge test -vvv
```

## Deploy locally

```sh
anvil &
forge script script/Deploy.s.sol:Deploy --rpc-url http://localhost:8545 --broadcast
```

The deploy script prints the registry and aggregator addresses; the web app
reads them from `apps/web/lib/contracts.json` (populated by `scripts/dev.sh`).
