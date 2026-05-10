"use client";

import { useEffect, useState } from "react";
import { createPublicClient, http, type Address, type Hex, type Log, parseAbiItem } from "viem";
import { foundry } from "viem/chains";
import { attestationRegistryAbi, loadAddresses } from "@/lib/contracts";
import { Stat } from "@/components/stat";

interface PublishedRoot {
  root: Hex;
  batchId: Hex;
  publisher: Address;
  timestamp: bigint;
}

export default function AttestationsPage() {
  const [registry, setRegistry] = useState<Address | null>(null);
  const [roots, setRoots] = useState<PublishedRoot[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const addrs = await loadAddresses();
        if (!addrs) {
          setError(
            "No contracts.json found. Run scripts/dev.sh to deploy locally.",
          );
          setLoading(false);
          return;
        }
        const client = createPublicClient({ chain: foundry, transport: http() });
        const event = parseAbiItem(
          "event RootPublished(bytes32 indexed root, bytes32 indexed batchId, address indexed publisher, uint256 timestamp)",
        );
        const logs: Log[] = await client.getLogs({
          address: addrs.AttestationRegistry as Address,
          event,
          fromBlock: 0n,
          toBlock: "latest",
        });
        const decoded: PublishedRoot[] = logs
          .map((l) => {
            const args = (l as unknown as { args: PublishedRoot }).args;
            return args;
          })
          .filter(Boolean);
        setRegistry(addrs.AttestationRegistry as Address);
        setRoots(decoded.reverse());
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  return (
    <section className="space-y-10">
      <header className="space-y-2">
        <h1 className="font-serif text-3xl tracking-tight">Attestations</h1>
        <p className="max-w-2xl text-sm text-navy-200/80">
          Each Tyche batch is a Merkle tree of signed simulation records. Roots are anchored on
          chain by the <span className="font-mono">AttestationRegistry</span>. With the chain root
          and an inclusion proof, anyone can verify a single record without seeing the others.
        </p>
      </header>

      <div className="grid grid-cols-1 gap-6 sm:grid-cols-3">
        <Stat label="Chain" value="Anvil" hint="local devnet, port 8545" />
        <Stat label="Registry" value={registry ? `${registry.slice(0, 10)}…` : "–"} />
        <Stat label="Roots anchored" value={roots.length.toString()} />
      </div>

      {loading && <p className="text-sm text-navy-200">Loading anchored roots…</p>}
      {error && <p className="text-sm text-ochre">{error}</p>}
      {!loading && !error && roots.length === 0 && (
        <p className="text-sm text-navy-200/80">
          No roots anchored yet. Use <span className="font-mono">tyche attest</span> and
          <span className="font-mono"> cast send</span> to publish one.
        </p>
      )}

      {roots.length > 0 && (
        <div className="overflow-hidden rounded-md border border-navy-700/60">
          <table className="min-w-full text-sm">
            <thead>
              <tr className="text-left text-xs uppercase tracking-widest text-navy-200/80">
                <th className="px-4 py-2">Root</th>
                <th className="px-4 py-2">Batch</th>
                <th className="px-4 py-2">Publisher</th>
                <th className="px-4 py-2 text-right">Timestamp</th>
              </tr>
            </thead>
            <tbody>
              {roots.map((r) => (
                <tr key={r.root} className="border-t border-navy-700/40 font-mono">
                  <td className="px-4 py-2 text-navy-200" title={r.root}>
                    {r.root.slice(0, 18)}…
                  </td>
                  <td className="px-4 py-2 text-navy-200" title={r.batchId}>
                    {r.batchId.slice(0, 12)}…
                  </td>
                  <td className="px-4 py-2 text-navy-200" title={r.publisher}>
                    {r.publisher.slice(0, 10)}…
                  </td>
                  <td className="px-4 py-2 text-right text-navy-200">
                    {new Date(Number(r.timestamp) * 1000).toISOString().slice(0, 19)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <ProofVerifier registry={registry} />
    </section>
  );
}

function ProofVerifier({ registry }: { registry: Address | null }) {
  const [root, setRoot] = useState<string>("");
  const [leaf, setLeaf] = useState<string>("");
  const [proof, setProof] = useState<string>("");
  const [result, setResult] = useState<string | null>(null);

  async function verify() {
    setResult(null);
    if (!registry) {
      setResult("registry address not loaded");
      return;
    }
    try {
      const client = createPublicClient({ chain: foundry, transport: http() });
      const proofArr = proof
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      const ok = await client.readContract({
        abi: attestationRegistryAbi,
        address: registry,
        functionName: "verifyInclusion",
        args: [root as Hex, leaf as Hex, proofArr as Hex[]],
      });
      setResult(ok ? "OK" : "Not included");
    } catch (e) {
      setResult(e instanceof Error ? e.message : String(e));
    }
  }

  return (
    <div className="space-y-3 rounded-md border border-navy-700/60 p-5">
      <h2 className="text-sm font-medium uppercase tracking-widest text-navy-200">
        Verify inclusion
      </h2>
      <input
        placeholder="Root (0x…)"
        value={root}
        onChange={(e) => setRoot(e.target.value)}
        className="w-full rounded-md border border-navy-700/60 bg-navy-700/30 px-3 py-2 font-mono text-xs"
      />
      <input
        placeholder="Leaf (0x…)"
        value={leaf}
        onChange={(e) => setLeaf(e.target.value)}
        className="w-full rounded-md border border-navy-700/60 bg-navy-700/30 px-3 py-2 font-mono text-xs"
      />
      <input
        placeholder="Proof, comma-separated 0x… values"
        value={proof}
        onChange={(e) => setProof(e.target.value)}
        className="w-full rounded-md border border-navy-700/60 bg-navy-700/30 px-3 py-2 font-mono text-xs"
      />
      <button
        onClick={verify}
        className="rounded-md bg-ochre px-4 py-2 text-sm font-medium text-canvas hover:bg-ochre-600"
      >
        Verify
      </button>
      {result && <p className="font-mono text-sm">{result}</p>}
    </div>
  );
}
