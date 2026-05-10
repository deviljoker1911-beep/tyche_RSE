/**
 * Contract addresses and ABI fragments for the Tyche on-chain layer.
 *
 * In dev, `scripts/dev.sh` deploys via Foundry and writes the resulting
 * addresses to `apps/web/public/contracts.json`. The page-level loader reads
 * that file at request time.
 */

import { z } from "zod";

export const ContractAddressesZ = z.object({
  AttestationRegistry: z.string().regex(/^0x[a-fA-F0-9]{40}$/),
  FederationAggregator: z.string().regex(/^0x[a-fA-F0-9]{40}$/),
});

export type ContractAddresses = z.infer<typeof ContractAddressesZ>;

export async function loadAddresses(): Promise<ContractAddresses | null> {
  try {
    const res = await fetch("/contracts.json", { cache: "no-store" });
    if (!res.ok) return null;
    return ContractAddressesZ.parse(await res.json());
  } catch {
    return null;
  }
}

export const attestationRegistryAbi = [
  {
    type: "function",
    name: "isPublished",
    stateMutability: "view",
    inputs: [{ name: "root", type: "bytes32" }],
    outputs: [{ type: "bool" }],
  },
  {
    type: "function",
    name: "publishedAt",
    stateMutability: "view",
    inputs: [{ name: "root", type: "bytes32" }],
    outputs: [{ type: "uint256" }],
  },
  {
    type: "function",
    name: "verifyInclusion",
    stateMutability: "view",
    inputs: [
      { name: "root", type: "bytes32" },
      { name: "leaf", type: "bytes32" },
      { name: "proof", type: "bytes32[]" },
    ],
    outputs: [{ type: "bool" }],
  },
  {
    type: "event",
    name: "RootPublished",
    inputs: [
      { indexed: true, name: "root", type: "bytes32" },
      { indexed: true, name: "batchId", type: "bytes32" },
      { indexed: true, name: "publisher", type: "address" },
      { indexed: false, name: "timestamp", type: "uint256" },
    ],
  },
] as const;
