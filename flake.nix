{
  description = "Union is a trust-minimized, zero-knowledge bridging protocol, designed for censorship resistance, extremely high security and usage in decentralized finance.";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-24.11";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
    };
    rust-overlay.url = "github:oxalica/rust-overlay";
    union = {
      url = "github:unionlabs/union/crane-stuff";
    };
  };
  outputs =
    inputs@{
      self,
      nixpkgs,
      flake-parts,
      union,
      rust-overlay,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      imports = [];

      perSystem =
        {
          inputs',
          pkgs,
          system,
          ...
        }:
        let
          dbg =
            value:
            builtins.trace (
              if value ? type && value.type == "derivation" then
                "derivation: ${value}"
              else
                pkgs.lib.generators.toPretty { } value
            ) value;

          crane = inputs'.union.packages.rust-lib.mkCrane {
            root = ./.;
            gitRev = (union.lib.getRepoMeta self).gitRev;
          };
        in
        {
          _module = {
            args = {
              inherit nixpkgs dbg;

              pkgs = nixpkgs.legacyPackages.${system}.appendOverlays [ rust-overlay.overlays.default ];
            };
          };

          packages = {
            liquidstaking-solo = crane.buildWasmContract "contracts/liquidstaking/liquidstaking-solo" {};
            liquidstaking = crane.buildWasmContract "contracts/liquidstaking/liquidstaking" {};
            reward = crane.buildWasmContract "contracts/reward" {};
            cw20-base = crane.buildWasmContract "contracts/cw20-base" {};
          };

          # checks = { };
        };
    };
}
