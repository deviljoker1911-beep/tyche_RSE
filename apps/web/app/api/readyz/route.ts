/**
 * Readiness probe.
 *
 * Today: identical to /healthz. M1G will probe TYCHE_API_URL availability
 * here so K8s can stop routing traffic to a web pod that has lost its
 * upstream.
 */
export const dynamic = "force-dynamic";

export async function GET() {
  const upstream = process.env.NEXT_PUBLIC_TYCHE_API_URL;
  if (!upstream) {
    return Response.json({ status: "ready", upstream: null });
  }
  try {
    const r = await fetch(new URL("/healthz", upstream), {
      signal: AbortSignal.timeout(1_500),
    });
    if (!r.ok) {
      return Response.json(
        { status: "degraded", upstream_status: r.status },
        { status: 503 },
      );
    }
    return Response.json({ status: "ready", upstream: r.status });
  } catch (err) {
    return Response.json(
      { status: "degraded", upstream_error: String(err) },
      { status: 503 },
    );
  }
}
