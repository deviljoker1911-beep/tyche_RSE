{
  description = "Tyche — federated risk simulation for European private credit";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    foundry = {
      url = "github:shazow/foundry.nix/monthly";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, foundry }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) foundry.overlay ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            cargo-watch
            cargo-nextest
            cargo-criterion
            wasm-pack
            wasm-bindgen-cli
            binaryen
            foundry-bin
            slither-analyzer
            nodejs_20
            nodePackages.pnpm
            (python3.withPackages (ps: with ps; [
              jupyterlab
              numpy
              pandas
              matplotlib
              scipy
            ]))
            jq
            tmux
            pkg-config
            openssl
          ];

          shellHook = ''
            echo "Tyche dev shell ready."
            echo "Run scripts/dev.sh to bring up the local stack."
          '';
        };
      });
}
