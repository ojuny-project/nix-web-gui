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

  /* @option widget = "table-list"; label = "Rows"; columns = [ "name" "enabled" ]; columnTypes = [ "str" "bool" ]; */
  services.exampleRows = [
    { name = "one"; enabled = true; }
  ];

  /* @option widget = "submodule"; label = "Nested settings"; */
  services.example = {
    /* @option widget = "switch"; label = "Nested switch"; */
    enable = true;

    /* @option widget = "text"; label = "Nested name"; */
    name = "nested";
  };
}
