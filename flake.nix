{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";

    # Dev tools
    treefmt-nix.url = "github:numtide/treefmt-nix";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
      imports = [
        inputs.treefmt-nix.flakeModule
      ];

      perSystem =
        {
          self,
          config,
          system,
          ...
        }:
        let
          overlays = [ inputs.rust-overlay.overlays.default ];
          pkgs = import inputs.nixpkgs {
            inherit system overlays;
          };
          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          rustToolchain = pkgs.rust-bin.stable."1.90.0".default;

          rust-toolchain = pkgs.symlinkJoin {
            name = "rust-toolchain";
            paths = [
              rustToolchain
              pkgs.cargo-watch
              pkgs.rust-analyzer
              pkgs.cargo-dist
              pkgs.cargo-tarpaulin
              pkgs.cargo-insta
              pkgs.cargo-machete
              pkgs.cargo-edit
            ];
          };
          buildInputs = [ ];
          nativeBuildInputs = with pkgs; [ installShellFiles ];
        in
        {
          # Rust package
          packages.default = pkgs.rustPlatform.buildRustPackage {
            inherit (cargoToml.package) name version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;

            RUST_BACKTRACE = "full";

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs;
            postInstall = ''
              install -Dm444 desktop/vault-tasks.desktop -t $out/share/applications
            ''
            + ''
              # vault-tasks tries to load a config file from ~/.config/ before generating completions
              export HOME="$(mktemp -d)"

              installShellCompletion --cmd vault-tasks \
                --bash <($out/bin/vault-tasks generate-completions bash) \
                --fish <($out/bin/vault-tasks generate-completions fish) \
                --zsh <($out/bin/vault-tasks generate-completions zsh)
            '';
          };

          # Rust dev environment
          devShells.default = pkgs.mkShell {
            inputsFrom = [
              config.treefmt.build.devShell
            ];
            RUST_BACKTRACE = "full";
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

            packages =
              nativeBuildInputs
              ++ buildInputs
              ++ [
                rust-toolchain
                pkgs.go
                pkgs.clippy
                pkgs.just
                pkgs.vhs
                (pkgs.python3.withPackages (python-pkgs: [
                  python-pkgs.ics
                ]))
              ];
          };

          # Add your auto-formatters here.
          # cf. https://numtide.github.io/treefmt/
          treefmt.config = {
            projectRootFile = "flake.nix";
            programs = {
              nixpkgs-fmt.enable = true;
              rustfmt.enable = true;
              toml-sort.enable = true;
            };
          };
        };
    };
}
