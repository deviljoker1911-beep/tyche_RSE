import type { NextConfig } from "next";

const config: NextConfig = {
  reactStrictMode: true,
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
