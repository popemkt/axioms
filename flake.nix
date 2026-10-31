{
  description = "axioms: learn any topic starting from what you already know";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
            rust-analyzer
            # links the SpacetimeDB module (wasm32-unknown-unknown)
            lld
            # the SpacetimeDB client SDK's websocket TLS (native-tls)
            openssl
            pkg-config
            # bundles the SpacetimeDB TypeScript SDK for the web UI, once
            esbuild
            nodejs
          ];
          # The host has unexplained hardware resets: keep builds gentle.
          CARGO_BUILD_JOBS = "4";
          shellHook = ''
            git config core.hooksPath .githooks
          '';
        };
      });
    };
}
