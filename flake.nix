{
  description = "Union is a trust-minimized, zero-knowledge bridging protocol, designed for censorship resistance, extremely high security and usage in decentralized finance.";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-24.11";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
    };
    rust-overlay.url = "github:oxalica/rust-overlay";
    union = {
      url = "github:unionlabs/union";
    };
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };
  outputs =
    inputs@{
      self,
      nixpkgs,
      flake-parts,
      union,
      rust-overlay,
      treefmt-nix,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      imports = [ treefmt-nix.flakeModule ];

      perSystem =
        {
          config,
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

          nightlyVersion = "2025-08-26";

          availableComponents = {
            rustc = "rustc";
            cargo = "cargo";
            rustfmt = "rustfmt";
            rust-std = "rust-std";
            rust-docs = "rust-docs";
            rust-analyzer = "rust-analyzer";
            clippy = "clippy";
            miri = "miri";
            rust-src = "rust-src";
            llvm-tools-preview = "llvm-tools-preview";
          };

          rust-dev-toolchain = pkgs.rust-bin.nightly.${nightlyVersion}.default.override {
            extensions = builtins.attrValues availableComponents;
            targets = [ "wasm32-unknown-unknown" ];
          };

          crane = inputs'.union.packages.rust-lib.mkCrane {
            root = ./.;
            inherit ((union.lib.getRepoMeta self)) gitRev;
          };

          cargoWorkspaceAttrs = {
            pname = "cargo-workspace";
            version = "0.0.0";
            src = crane.cargoWorkspaceSrc;

            cargoTestExtraArgs = "--workspace --no-fail-fast";
            cargoClippyExtraArgs = "--workspace --tests -- -Dwarnings";

            CARGO_PROFILE = "dev";

            buildInputs = [
              pkgs.pkg-config
              pkgs.openssl
            ];
            nativeBuildInputs = [ ];
          };

          cargoArtifacts = crane.lib.buildDepsOnly cargoWorkspaceAttrs;
        in
        {
          _module = {
            args = {
              inherit nixpkgs dbg;

              pkgs = nixpkgs.legacyPackages.${system}.appendOverlays [ rust-overlay.overlays.default ];
            };
          };

          packages = {
            liquidstaking-babylon = crane.buildWasmContract "contracts/liquidstaking/liquidstaking-babylon" { };
            liquidstaking-union = crane.buildWasmContract "contracts/liquidstaking/liquidstaking-union" { };
            reward = crane.buildWasmContract "contracts/reward" { };
            cw20-base = crane.buildWasmContract "contracts/cw20-base" { };
          };

          checks = {
            cargo-workspace-clippy = crane.lib.cargoClippy (cargoWorkspaceAttrs // { inherit cargoArtifacts; });
            cargo-workspace-test = crane.lib.cargoTest (cargoWorkspaceAttrs // { inherit cargoArtifacts; });
          };

          devShells.default = pkgs.mkShell {
            name = "union-devShell";
            buildInputs =
              [ rust-dev-toolchain ]
              ++ (with pkgs; [
                jq
                marksman
                nil
                protobuf
                yq
                wasm-tools
                binaryen
                openssl
                pkg-config
              ]);
            nativeBuildInputs = [
              config.treefmt.build.wrapper
            ] ++ pkgs.lib.attrsets.attrValues config.treefmt.build.programs;

            RUST_SRC_PATH = "${rust-dev-toolchain}/lib/rustlib/src/rust/library";
            PROTOC = "${pkgs.protobuf}/bin/protoc";
          };

          treefmt = {
            package = pkgs.treefmt;
            projectRootFile = "flake.nix";
            programs = {
              rustfmt = {
                enable = true;
                package = rust-dev-toolchain;
              };
              taplo.enable = true;
              yamlfmt = {
                enable = true;
                package = pkgs.yamlfmt;
              };
              mdformat = {
                enable = true;
                package = pkgs.mdformat;
              };
              shellcheck = {
                enable = true;
                package = pkgs.shellcheck;
              };
              nixfmt-rfc-style = {
                enable = true;
                package = pkgs.nixfmt-rfc-style;
              };
              statix = {
                enable = true;
                package = pkgs.statix;
              };
              deadnix = {
                enable = true;
                package = pkgs.deadnix;
              };
            };
            settings = {
              formatter = {
                nixfmt-rfc-style = {
                  options = [ ];
                  includes = [ "*.nix" ];
                };
                statix.options = [ "explain" ];
                mdformat.options = [ "--number" ];
                deadnix.options = [ "--no-lambda-pattern-names" ];
                shellcheck.options = [
                  "--shell=bash"
                  "--check-sourced"
                ];
                yamlfmt.options = [
                  "-formatter"
                  "retain_line_breaks=true"
                ];
              };
              global = {
                hidden = true;
                excludes = [
                  "*.ttf"
                  "*.png"
                  "*.prv"
                  "*.bin"
                  "*.jpg"
                  "*.svg"
                  "*.jpeg"
                  "*.lock"
                  ".git/**"
                  ".ignore"
                  "LICENSE"
                  "LICENSE*"
                  "**/LICENSE"
                  "CODEOWNERS"
                  ".gitignore"
                  "*.splinecode"
                  "**/.gitignore"
                  ".gitattributes"
                  ".github/**/*.sh"
                  ".github/**/*.md"
                  "**/.gitattributes"
                  ".git-blame-ignore-revs"
                ];
              };
            };
          };
        };
    };
}
