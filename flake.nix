{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/25.11";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
  };
  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
      perSystem =
        { system, ... }:
        let
          pkgs = import inputs.nixpkgs { inherit system; };
          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
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
            postInstall = ''install -Dm444 desktop/vault-tasks.desktop -t $out/share/applications '' + ''
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
            packages = with pkgs; [
              cargo
              rustc
              clippy
              rustfmt
              rust-analyzer
              cargo-dist
              cargo-tarpaulin
              cargo-insta
              cargo-machete
              cargo-edit
              just
              vhs
            ];
          };
        };
    };
}
