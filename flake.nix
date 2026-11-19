{
  description = "nix-web-gui: a web settings GUI over the nix-settings-gui engine";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    # Release builds use the tagged revision of the upstream engine.
    nix-settings-gui.url = "github:ojuny-project/nix-settings-gui/3f3a980d5186321ef4c01c8e064a17a0b5b10b5d";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      nix-settings-gui,
    }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;

        frontend = pkgs.callPackage ./nix/frontend.nix { };

        launchers = import ./nix/apps.nix {
          inherit pkgs;
          package = self.packages.${system}.default;
        };

        mkPackage =
          {
            withOjunyWidgets ? true,
            pname ? "nix-web-gui",
          }:
          pkgs.callPackage ./nix/package.nix {
            inherit pname withOjunyWidgets;
            frontend = pkgs.callPackage ./nix/frontend.nix { inherit withOjunyWidgets; };
            nixSettingsGui = nix-settings-gui.outPath;
          };
      in
      {
        packages = {
          default = mkPackage { };
          nix-web-gui = mkPackage { };
          nix-web-gui-minimal = mkPackage {
            withOjunyWidgets = false;
            pname = "nix-web-gui-minimal";
          };
          container = import ./nix/container.nix {
            inherit pkgs;
            package = self.packages.${system}.default;
          };
        };

        apps.default = {
          type = "app";
          program = "${launchers.production}/bin/nix-web-gui";
        };

        apps.demo = {
          type = "app";
          program = "${launchers.demo}/bin/nix-web-gui-demo";
        };

        checks = {
          default = self.packages.${system}.default;
          minimal = self.packages.${system}.nix-web-gui-minimal;
          boundaries = pkgs.runCommand "check-boundaries" { } ''
            cd ${self}
            ${pkgs.bash}/bin/bash tests/check-boundaries.sh
            touch $out
          '';
          minimal-flag = pkgs.runCommand "check-minimal-flag" { } ''
            grep -Rq "This Ojuny widget is not available in the minimal build." \
              ${self.packages.${system}.nix-web-gui-minimal}/share/nix-web-gui/dist
            touch $out
          '';
          minimal-renderer = pkgs.runCommand "check-minimal-renderer" { } ''
            default_dist=${self.packages.${system}.default}/share/nix-web-gui/dist
            dist=${self.packages.${system}.nix-web-gui-minimal}/share/nix-web-gui/dist
            grep -Rq "Variable name is required" "$default_dist" \
              || { echo "the default build no longer contains the Docker editor" >&2; exit 1; }
            grep -Rq "Cannot read plugin schema" "$default_dist" \
              || { echo "the default build no longer contains the plugin editor" >&2; exit 1; }
            if grep -Rq "Variable name is required" "$dist"; then
              echo "the Docker environment editor shipped in the minimal build" >&2
              exit 1
            fi
            if grep -Rq "Cannot read plugin schema" "$dist"; then
              echo "the plugin settings editor shipped in the minimal build" >&2
              exit 1
            fi
            touch $out
          '';
          parity = pkgs.runCommand "check-parity" { } ''
            cd ${self}
            ${pkgs.bash}/bin/bash tests/check-parity.sh .
            touch $out
          '';
        } // lib.optionalAttrs (system == "x86_64-linux") {
          nixos-test-auth = import ./tests/nixos-test-auth.nix {
            inherit self pkgs system;
          };
          ui-smoke = pkgs.runCommand "check-ui-smoke" {
            nativeBuildInputs = [ pkgs.curl ];
            FONTCONFIG_FILE = "${pkgs.makeFontsConf { fontDirectories = [ pkgs.dejavu_fonts ]; }}";
          } ''
            ${pkgs.bash}/bin/bash ${self}/tests/ui-check.sh \
              ${self.packages.${system}.default} \
              ${pkgs.chromium} \
              ${pkgs.nodejs} \
              ${self}/tests/fixtures/ui-flake \
              ${self}/tests/ui-smoke.mjs
            touch $out
          '';
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            nodejs
            pkg-config
            nix
            nixd
            nixfmt
            git
            curl
            mold
          ];

          buildInputs = with pkgs; [
            pam
            openssl
            llvmPackages.libclang
            gtk4
            libadwaita
            glib
          ];

          nativeBuildInputs = [ pkgs.rustPlatform.bindgenHook ];
          RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.lib.getDev pkgs.pam}/include -I${pkgs.glibc.dev}/include";
        };
      }
    )
    // {
      nixosModules.default = import ./modules/nixos.nix self;
      nixosModules.nix-web-gui = import ./modules/nixos.nix self;
      nixosModules.ojuny-theme = import ./modules/ojuny-theme.nix;
    };
}
