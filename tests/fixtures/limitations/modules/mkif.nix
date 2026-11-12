/* @menu displayname = "If"; */
{ lib, ... }: {
  config = lib.mkIf true {
    /* @option widget = "text"; label = "Visible if option"; */
    time.timeZone = "UTC";
  };
}
