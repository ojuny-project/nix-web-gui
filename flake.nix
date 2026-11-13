{
  description = "nix-web-gui: a web settings GUI over the nix-settings-gui engine";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    # Replace with github:ojuny-project/nix-settings-gui for a release build.
    nix-settings-gui.url = "git+file:///home/den/repos/nix-settings-gui";
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
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/nix-web-gui";
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
        } // lib.optionalAttrs (system == "x86_64-linux") {
          nixos-test-auth = import ./tests/nixos-test-auth.nix {
            inherit self pkgs system;
          };
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
