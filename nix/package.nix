{
  pkgs,
  lib,
  rustPlatform,
  pkg-config,
  openssl,
  pam,
  libclang,
  glibc,
  gtk4,
  libadwaita,
  glib,
  frontend,
  nixSettingsGui,
  withOjunyWidgets ? true,
  pname ? "nix-web-gui",
}:
let
  cleanSource = lib.cleanSourceWith {
    src = ../.;
    filter =
      path: _type:
      let
        base = baseNameOf path;
      in
      !(builtins.elem base [
        ".git"
        "target"
        "node_modules"
        "dist"
        "result"
      ]) && !(lib.hasPrefix "result-" base);
  };
in
rustPlatform.buildRustPackage {
  inherit pname;
  version = "0.1.0";
  src = pkgs.runCommand "nix-web-gui-src" { } ''
    mkdir -p $out/vendor
    cp -r ${cleanSource}/. $out/
    cp -r ${nixSettingsGui}/. $out/vendor/nix-settings-gui
    chmod -R u+w $out
    sed -i 's|path = "../../../nix-settings-gui"|path = "../vendor/nix-settings-gui"|' $out/engine/Cargo.toml
  '';

  cargoLock.lockFile = ../Cargo.lock;
  cargoBuildFlags = [ "-p" "nix-web-gui-server" ]
    ++ lib.optionals (!withOjunyWidgets) [
      "--no-default-features"
      "--features"
      "pam"
    ];

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
  ];
  buildInputs = [
    openssl
    pam
    gtk4
    libadwaita
    glib
  ];

  LIBCLANG_PATH = "${lib.getLib libclang}/lib";
  BINDGEN_EXTRA_CLANG_ARGS = "-I${lib.getDev pam}/include -I${glibc.dev}/include";

  doCheck = true;

  postInstall = ''
    mkdir -p $out/share/nix-web-gui
    cp -r ${frontend}/dist $out/share/nix-web-gui/dist
  '';

  meta = with lib; {
    description = "Web settings GUI over the nix-settings-gui engine";
    mainProgram = "nix-web-gui";
    license = licenses.gpl3Plus;
    platforms = platforms.linux;
  };
}
