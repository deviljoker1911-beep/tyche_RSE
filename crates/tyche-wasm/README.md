# tyche-wasm

WebAssembly bindings exposing the Tyche simulation and attestation core to a
JavaScript host.

Build:

```sh
wasm-pack build --target web --out-dir ../../apps/web/public/wasm crates/tyche-wasm
```

The resulting `pkg/` is consumed by `apps/web` for in-browser simulation.
