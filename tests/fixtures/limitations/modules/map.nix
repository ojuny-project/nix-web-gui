/* @menu displayname = "Map"; */
{ lib, ... }: {
  config = map (name: {
    /* @option widget = "text"; label = "Hidden map option"; */
    networking.domain = name;
  }) [ "hidden-map" ];
}
