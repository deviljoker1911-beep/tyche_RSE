import { http, createConfig } from "wagmi";
import { foundry } from "wagmi/chains";

/**
 * Wagmi config for the spike. Targets a local Anvil chain on port 8545.
 * Production deployments will replace this with a multi-chain config that
 * also speaks Polygon zkEVM testnet.
 */
export const wagmiConfig = createConfig({
  chains: [foundry],
  transports: {
    [foundry.id]: http("http://127.0.0.1:8545"),
  },
});
