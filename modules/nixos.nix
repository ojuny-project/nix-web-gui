self:
{
  config,
  lib,
  pkgs,
  ...
}:
with lib;
let
  cfg = config.services.nix-web-gui;

  package =
    if cfg.withOjunyWidgets
    then self.packages.${pkgs.stdenv.hostPlatform.system}.default
    else self.packages.${pkgs.stdenv.hostPlatform.system}.nix-web-gui-minimal;

  allowedGroupArgs = concatMapStringsSep " " (group: "--allowed-group ${escapeShellArg group}") cfg.allowedGroups;
  flakeArg = optionalString (cfg.flakePath != null) "--flake ${escapeShellArg cfg.flakePath}";
  pageArg = optionalString (cfg.startupPage != null) "--page ${escapeShellArg cfg.startupPage}";
in
{
  options.services.nix-web-gui = {
    enable = mkEnableOption "nix-web-gui, a web settings GUI over the nix-settings-gui engine";

    package = mkOption {
      type = types.package;
      default = package;
      description = "The nix-web-gui package to use.";
    };

    withOjunyWidgets = mkOption {
      type = types.bool;
      default = true;
      description = "Build with the Ojuny-specific bar, dock, menu, plugins, grid, and docker widgets.";
    };

    listenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address the HTTP listener binds to. Keep loopback unless a TLS reverse proxy is in front.";
    };

    port = mkOption {
      type = types.port;
      default = 8787;
      description = "HTTP listener port.";
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Open the listener port in the firewall.";
    };

    allowRemote = mkOption {
      type = types.bool;
      default = false;
      description = "Allow binding a non-loopback address. Requires a non-empty allowedGroups list.";
    };

    allowedGroups = mkOption {
      type = types.listOf types.str;
      default = [ "wheel" ];
      description = "Device groups allowed to sign in. An empty list allows every device account.";
    };

    pamService = mkOption {
      type = types.str;
      default = "nix-web-gui";
      description = "PAM service name used for device-account authentication.";
    };

    flakePath = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "/home/den/repos/nixos-configs/dendrite";
      description = "Default flake or configuration directory offered to a signed-in browser.";
    };

    startupPage = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "Theme";
      description = "Optional regex for the first menu selected after a flake opens.";
    };

    theme = mkOption {
      type = types.str;
      default =
        let
          palette = config.ojuny.style.palette or null;
        in
        if palette != null then palette else "everforest-dark-hard";
      description = "Base16 scheme name from pkgs.base16-schemes. The theming module feeds this from config.ojuny.style.palette.";
    };

    themeFile = mkOption {
      type = types.path;
      default = "/etc/nix-web-gui/theme";
      description = "File the server reads the active base16 scheme name from.";
    };

    sessionTtlSeconds = mkOption {
      type = types.ints.positive;
      default = 43200;
      description = "Browser session lifetime.";
    };

    maxSessions = mkOption {
      type = types.ints.positive;
      default = 16;
      description = "Maximum number of live browser sessions. The oldest session is dropped first.";
    };

    secureCookie = mkOption {
      type = types.bool;
      default = false;
      description = "Always set the Secure flag on the session cookie. Enable this when a TLS reverse proxy terminates HTTPS.";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion =
          cfg.listenAddress == "127.0.0.1"
          || cfg.listenAddress == "::1"
          || (cfg.allowRemote && cfg.allowedGroups != [ ]);
        message = "nix-web-gui: set allowRemote and a non-empty allowedGroups list before binding a non-loopback address.";
      }
    ];

    security.pam.services.system-auth = { };

    security.pam.services.nix-web-gui.text = ''
      auth include system-auth
      account include system-auth
      password include system-auth
      session include system-auth
    '';

    environment.etc."nix-web-gui/theme".text = cfg.theme + "\n";

    systemd.services.nix-web-gui = {
      description = "nix-web-gui web settings service";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      serviceConfig = {
        ExecStart = ''
          ${cfg.package}/bin/nix-web-gui serve \
            --listen ${cfg.listenAddress}:${toString cfg.port} \
            --static-dir ${cfg.package}/share/nix-web-gui/dist \
            --theme-file ${cfg.themeFile} \
            --pam-service ${escapeShellArg cfg.pamService} \
            ${flakeArg} \
            --session-ttl-secs ${toString cfg.sessionTtlSeconds} \
            --max-sessions ${toString cfg.maxSessions} \
            ${optionalString cfg.secureCookie "--secure-cookie"} \
            ${allowedGroupArgs}
        '';
        User = "root";
        Restart = "on-failure";
        RestartSec = "2s";
        NoNewPrivileges = false;
      };
    };

    networking.firewall.allowedTCPPorts = mkIf (cfg.openFirewall && cfg.listenAddress != "127.0.0.1") [ cfg.port ];
  };
}
