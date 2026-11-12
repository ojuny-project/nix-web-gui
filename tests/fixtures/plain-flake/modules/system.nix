/* @menu
   displayname = "System";
   description = "System settings";
   category = "Core";
   order = 10;
*/
{ lib, ... }: {
  /* @option widget = "text"; label = "Hostname"; description = "Network name"; */
  networking.hostName = "nixos";

  /* @option widget = "switch"; label = "Enable PipeWire"; */
  services.pipewire.enable = true;

  /* @option widget = "dropdown"; label = "Desktop"; choices = [ "gnome" "plasma" ]; */
  services.xserver.desktopManager = "gnome";
}
