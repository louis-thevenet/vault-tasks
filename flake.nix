{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
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
            paths =
              with pkgs;
              [
                rust-analyzer
                cargo-dist
                cargo-tarpaulin
                cargo-insta
                cargo-machete
                cargo-edit
              ]
              ++ [ rustToolchain ];
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
            RUST_BACKTRACE = "full";
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

            packages =
              nativeBuildInputs
              ++ buildInputs
              ++ (with pkgs; [
                clippy
                just
                vhs
              ])
              ++ [ rust-toolchain ];
          };
        };
    };
}
