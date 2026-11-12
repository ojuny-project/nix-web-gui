/* @menu displayname = "Merge"; */
{ lib, ... }: {
  config = lib.mkMerge [
    {
      /* @option widget = "text"; label = "Hidden merge option"; */
      networking.hostName = "hidden-merge";
    }
  ];
}
