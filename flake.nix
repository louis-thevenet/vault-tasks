{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";

    # Dev tools
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = import inputs.systems;
      imports = [
        inputs.treefmt-nix.flakeModule
      ];
      perSystem = {
        config,
        self',
        pkgs,
        lib,
        system,
        ...
      }: let
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        rust-toolchain = pkgs.symlinkJoin {
          name = "rust-toolchain";
          paths = with pkgs; [
            rustc
            cargo
            cargo-watch
            rust-analyzer
            rustPlatform.rustcSrc
            cargo-dist
            cargo-tarpaulin
            cargo-insta
            cargo-machete
          ];
        };

        buildInputs = with pkgs; [];
        nativeBuildInputs = with pkgs; [];
      in {
        # Rust package
        packages.default = pkgs.rustPlatform.buildRustPackage {
          inherit (cargoToml.package) name version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          RUST_BACKTRACE = "full";

          nativeBuildInputs = nativeBuildInputs;
          buildInputs = buildInputs;
          postInstall = "install -Dm444 desktop/vault-tasks.desktop -t $out/share/applications";
        };

        # Rust dev environment
        devShells.default = pkgs.mkShell {
          inputsFrom = [
            config.treefmt.build.devShell
          ];
          RUST_BACKTRACE = "full";
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

          nativeBuildInputs = nativeBuildInputs;
          buildInputs = buildInputs ++ [rust-toolchain pkgs.clippy];
        };

        # Add your auto-formatters here.
        # cf. https://numtide.github.io/treefmt/
        treefmt.config = {
          projectRootFile = "flake.nix";
          programs = {
            nixpkgs-fmt.enable = true;
            rustfmt.enable = true;
          };
        };
      };
    };
}
