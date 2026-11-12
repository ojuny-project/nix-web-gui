{
  lib,
  buildNpmPackage,
  nodejs,
  withOjunyWidgets ? true,
}:
buildNpmPackage {
  pname = "nix-web-gui-frontend";
  version = "0.1.0";
  src = ../frontend;

  npmDepsHash = "sha256-mv5ndm1ZVIFwAMc6nlJvhz+JJrSBIl2NAyezxsH3gaw=";
  npmBuildScript = "build";
  nativeBuildInputs = [ nodejs ];

  VITE_OJUNY_WIDGETS = if withOjunyWidgets then "1" else "0";

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r packages/app/dist $out/dist
    runHook postInstall
  '';

  meta = with lib; {
    description = "Browser frontend for nix-web-gui";
    license = licenses.gpl3Plus;
  };
}
