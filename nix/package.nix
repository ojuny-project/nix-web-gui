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
        "tests"
        "docs"
        ".github"
        "TODO.md"
        "README.md"
      ]) && !(lib.hasPrefix "result-" base);
  };
in
rustPlatform.buildRustPackage {
  inherit pname;
  version = "0.1.0";
  src = cleanSource;

  cargoLock = {
    lockFile = ../Cargo.lock;
    outputHashes = {
      "nix-settings-gui-0.1.0" = "sha256-sx/9TIqdp4U/9ioECBB/mK3nWoVOlr0JZ8zfsuXqpC8=";
    };
  };
  cargoBuildFlags = [ "-p" "nix-web-gui-server" ]
    ++ lib.optionals (!withOjunyWidgets) [
      "--no-default-features"
      "--features"
      "pam"
    ];

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
    pkgs.makeWrapper
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

  doCheck = false;

  postInstall = ''
    mkdir -p $out/share/nix-web-gui
    cp -r ${frontend}/dist $out/share/nix-web-gui/dist
  '';

  postFixup = ''
    wrapProgram $out/bin/nix-web-gui \
      --prefix PATH : ${
        lib.makeBinPath [
          pkgs.nix
          pkgs.nixd
          pkgs.nixfmt
          pkgs.git
          pkgs.curl
          pkgs.libxcrypt
          pkgs.openssl
          pkgs.sbctl
          pkgs.systemd
          pkgs.wlr-randr
        ]
      }
  '';

  meta = with lib; {
    description = "Web settings GUI over the nix-settings-gui engine";
    mainProgram = "nix-web-gui";
    license = licenses.gpl3Plus;
    platforms = platforms.linux;
  };
}
