{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable-small";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    naersk.url = "github:nix-community/naersk";
    # Dev tools
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake
      {
        inherit inputs;
      }
      {
        systems = import inputs.systems;
        imports = [
          inputs.treefmt-nix.flakeModule
        ];
        perSystem =
          {
            config,
            self',
            pkgs,
            lib,
            ...
          }:
          let
            naerskLib = pkgs.callPackage inputs.naersk { };

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
                cargo-edit
                cargo-tauri
              ];
            };

            buildInputsTauri = with pkgs; [
              at-spi2-atk
              atkmm
              cairo
              gdk-pixbuf
              glib
              gobject-introspection
              gobject-introspection.dev
              gtk3
              harfbuzz
              librsvg
              libsoup_3
              pango
              webkitgtk_4_1
              webkitgtk_4_1.dev
            ];
            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
          in
          rec {
            packages.default = packages.vault-tasks;
            packages.vault-tasks = naerskLib.buildPackage rec {
              name = "vault-tasks";
              src = ./.;
              cargoBuildOptions =
                x:
                x
                ++ [
                  "-p"
                  name
                ];
              postInstall = "install -Dm444 desktop/vault-tasks.desktop -t $out/share/applications";
            };

            # Does not work yet
            packages.vault-tasks-gui = naerskLib.buildPackage rec {
              name = "vault-tasks-gui";
              src = ./.;
              cargoBuildOptions =
                x:
                x
                ++ [
                  "-p"
                  name
                ];

              OPENSSL_NO_VENDOR = 1;
              # LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [ openssl_3 ]);
              OPENSSL_LIB_DIR = "${pkgs.lib.getLib pkgs.openssl_3}/lib";
              OPENSSL_DIR = "${pkgs.openssl_3.dev}";
              PKG_CONFIG_PATH =
                with pkgs;
                "${glib.dev}/lib/pkgconfig:${libsoup_3.dev}/lib/pkgconfig:${webkitgtk_4_1.dev}/lib/pkgconfig:${at-spi2-atk.dev}/lib/pkgconfig:${gtk3.dev}/lib/pkgconfig:${gdk-pixbuf.dev}/lib/pkgconfig:${cairo.dev}/lib/pkgconfig:${pango.dev}/lib/pkgconfig:${harfbuzz.dev}/lib/pkgconfig";

              nativeBuildInputs = nativeBuildInputs;
              buildInputs = buildInputsTauri;
            };

            # Rust dev environment
            devShells.default = pkgs.mkShell {
              OPENSSL_NO_VENDOR = 1;
              # LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [ openssl_3 ]);
              OPENSSL_LIB_DIR = "${pkgs.lib.getLib pkgs.openssl_3}/lib";
              OPENSSL_DIR = "${pkgs.openssl_3.dev}";
              PKG_CONFIG_PATH =
                with pkgs;
                "${glib.dev}/lib/pkgconfig:${libsoup_3.dev}/lib/pkgconfig:${webkitgtk_4_1.dev}/lib/pkgconfig:${at-spi2-atk.dev}/lib/pkgconfig:${gtk3.dev}/lib/pkgconfig:${gdk-pixbuf.dev}/lib/pkgconfig:${cairo.dev}/lib/pkgconfig:${pango.dev}/lib/pkgconfig:${harfbuzz.dev}/lib/pkgconfig";

              inputsFrom = [
                config.treefmt.build.devShell
              ];
              RUST_BACKTRACE = "full";
              RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;

              nativeBuildInputs = nativeBuildInputs;
              buildInputs = buildInputsTauri ++ [
                rust-toolchain
                pkgs.clippy
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
