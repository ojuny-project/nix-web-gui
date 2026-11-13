/* @menu displayname = "If"; */
{ lib, ... }: {
  /* @option widget = "text"; label = "Plain option"; */
  i18n.defaultLocale = "en_US.UTF-8";

  config = lib.mkIf true {
    /* @option widget = "text"; label = "Hidden if option"; */
    time.timeZone = "UTC";
  };
}
