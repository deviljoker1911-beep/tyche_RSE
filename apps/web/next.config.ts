import type { NextConfig } from "next";

const config: NextConfig = {
  reactStrictMode: true,
  // Required for `docker/Dockerfile.web` — produces a single-binary-style
  // bundle under `.next/standalone/`. Removing this breaks the production
  // image.
  output: "standalone",
  experimental: {
    typedRoutes: true,
  },
  // The wasm crate emits a `.wasm` asset that we serve from `/wasm/`.
  webpack(cfg) {
    cfg.experiments = { ...cfg.experiments, asyncWebAssembly: true };
    return cfg;
  },
};

export default config;
