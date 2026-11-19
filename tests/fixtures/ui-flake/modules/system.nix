/* @menu
   displayname = "System";
   description = "Browser check fixture";
   category = "Core";
   order = 10;
*/
{ ... }: {
  /* @option widget = "users"; label = "Users"; */
  users.users = {
    alice = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
      hashedPassword = "$y$fixture";
    };
  };

  home-manager.users = {
    alice = {
      home.stateVersion = "24.05";
    };
  };

  /* @option widget = "keybinds"; label = "Binds"; */
  keybinds = {
    reload = {
      modifier = [ "Mod4" ];
      keys = [ "r" ];
      display = "Reload";
    };
  };

  /* @option widget = "bar"; label = "Bar"; */
  bar = {
    left = [
      { id = "clock"; }
    ];
    center = [ "clock" ];
  };

  /* @option widget = "monitors"; label = "Monitors"; */
  monitors = [
    {
      name = "eDP-1";
      width = 1920;
      height = 1080;
      xPos = 0;
      yPos = 0;
    }
  ];

  /* @option widget = "grid-configuration"; label = "Grid"; gridColumns = 6; gridEnabled = true; gridRegistry = [ "clock" "weather" ]; */
  controlCenterWidgets = [
    {
      id = "clock";
      col = 0;
      row = 0;
      w = 2;
      h = 2;
    }
  ];

  /* @option widget = "text"; label = "Hostname"; */
  networking.hostName = "nixos";

  /* @option widget = "file"; label = "Background"; fileDir = "./assets"; fileExtensions = [ "png" ]; */
  services.background = "old.png";

  /* @option widget = "text"; label = "Apply target"; applyCommand = "touch applied.txt"; */
  services.applyTarget = "before";
}
