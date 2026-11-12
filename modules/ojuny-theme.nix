{ config, lib, ... }:
let
  palette = config.ojuny.style.palette or "everforest-dark-hard";
  highContrast = config.ojuny.accessibility.highContrast or false;
  highContrastPalette = config.ojuny.accessibility.highContrastPalette or palette;
  webTheme = if highContrast then highContrastPalette else palette;
in
{
  # The theming module owns the active base16 scheme. When nix-web-gui is
  # imported, the same name becomes the browser palette. No color translation
  # is needed because both sides use the tinted base16 collection.
  config = lib.mkIf (config.services ? nix-web-gui) {
    services.nix-web-gui.theme = lib.mkDefault webTheme;
  };
}
