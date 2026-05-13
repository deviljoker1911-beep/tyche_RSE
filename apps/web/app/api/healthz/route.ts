/**
 * Liveness probe.
 *
 * Returns 200 unconditionally if the Next.js server is responsive. Used by
 * the Docker HEALTHCHECK in `docker/Dockerfile.web` and by Kubernetes
 * liveness probes (manifest lands in M1D).
 */
export const dynamic = "force-dynamic";

export function GET() {
  return Response.json({ status: "ok" });
}
