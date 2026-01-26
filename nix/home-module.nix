# Home Manager module for HT32 Panel
#
# This module allows running the HT32 Panel daemon as a user service.
#
# Example usage in home.nix:
#   services.ht32-panel = {
#     enable = true;
#     package = inputs.ht32-panel.packages.${system}.default;
#   };
#
# For hardware access, also add to your NixOS configuration.nix:
#   imports = [ inputs.ht32-panel.nixosModules.udevRules ];
#   services.ht32-panel.udevRules.enable = true;
#
# Or use the combined module that reads from Home Manager config.

{ config, lib, pkgs, osConfig ? null, ... }:

let
  cfg = config.services.ht32-panel;
  settingsFormat = pkgs.formats.toml { };

  configFile = settingsFormat.generate "config.toml" ({
    web = {
      enable = cfg.web.enable;
      listen = cfg.web.listen;
    };
    dbus = {
      bus = cfg.dbus.bus;
    };
    theme = cfg.theme;
    poll = cfg.poll;
    refresh = cfg.refresh;
    heartbeat = cfg.heartbeat;
    devices = {
      lcd = cfg.devices.lcd;
      led = cfg.devices.led;
    };
    canvas = {
      width = cfg.canvas.width;
      height = cfg.canvas.height;
    };
  } // cfg.extraSettings);
in
{
  options.services.ht32-panel = {
    enable = lib.mkEnableOption "HT32 Panel daemon for LCD and LED control (user service)";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The ht32-panel daemon package to use.";
    };

    cli.package = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = null;
      description = "The ht32-panel CLI package to install. If null, uses the main package.";
    };

    web = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable the web interface.";
      };

      listen = lib.mkOption {
        type = lib.types.str;
        default = "[::1]:8686";
        description = "Address and port for the web interface.";
      };
    };

    dbus = {
      bus = lib.mkOption {
        type = lib.types.enum [ "auto" "session" "system" ];
        default = "session";
        description = ''
          Which D-Bus bus to use.
          - "session": Use the session bus (recommended for user services).
          - "system": Use the system bus (requires system-level D-Bus policy).
          - "auto": Try session bus first, fall back to system bus.
        '';
      };
    };

    theme = lib.mkOption {
      type = lib.types.str;
      default = "themes/default.toml";
      description = "Path to the theme configuration file.";
    };

    poll = lib.mkOption {
      type = lib.types.int;
      default = 500;
      description = "Render loop poll interval in milliseconds.";
    };

    refresh = lib.mkOption {
      type = lib.types.int;
      default = 1600;
      description = "Display refresh rate in milliseconds.";
    };

    heartbeat = lib.mkOption {
      type = lib.types.int;
      default = 60000;
      description = "Heartbeat interval in milliseconds.";
    };

    devices = {
      lcd = lib.mkOption {
        type = lib.types.str;
        default = "auto";
        description = "LCD device path or 'auto' for auto-detection.";
      };

      led = lib.mkOption {
        type = lib.types.str;
        default = "/dev/ttyUSB0";
        description = ''
          Serial port path for LED controller.
          Note: LED theme, intensity, and speed are stored in the state directory.
          Use `ht32panelctl led set <theme>` to change them.
        '';
      };
    };

    canvas = {
      width = lib.mkOption {
        type = lib.types.int;
        default = 320;
        description = "Canvas width in pixels.";
      };

      height = lib.mkOption {
        type = lib.types.int;
        default = 170;
        description = "Canvas height in pixels.";
      };
    };

    applet = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable the system tray applet for desktop environments.";
      };

      package = lib.mkOption {
        type = lib.types.nullOr lib.types.package;
        default = null;
        description = "The ht32-panel-applet package to use.";
      };

      autostart = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Whether to autostart the applet on login.";
      };
    };

    extraSettings = lib.mkOption {
      type = settingsFormat.type;
      default = { };
      description = "Additional settings to include in the configuration file.";
    };

    udevRules = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Whether to request udev rules for hardware access.
          When enabled, you must also import the NixOS udev rules module
          in your system configuration for this to take effect.
        '';
      };

      group = lib.mkOption {
        type = lib.types.str;
        default = "users";
        description = "Group to grant access to hardware devices.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    # Assert that udev rules are configured at the NixOS level if requested
    warnings = lib.optional (cfg.udevRules.enable && osConfig != null && !(osConfig.services.ht32-panel.udevRules.enable or false)) ''
      ht32-panel: udevRules.enable is true but NixOS udev rules are not configured.
      Add to your NixOS configuration:
        services.ht32-panel.udevRules = {
          enable = true;
          group = "${cfg.udevRules.group}";
        };
    '';

    # Add packages to user environment
    home.packages = [ cfg.package ]
      ++ lib.optional (cfg.cli.package != null) cfg.cli.package
      ++ lib.optional (cfg.applet.enable && cfg.applet.package != null) cfg.applet.package;

    # D-Bus session service file for on-demand activation
    xdg.dataFile."dbus-1/services/org.ht32panel.Daemon.service" = lib.mkIf (cfg.dbus.bus != "system") {
      text = ''
        [D-BUS Service]
        Name=org.ht32panel.Daemon
        Exec=${cfg.package}/bin/ht32paneld ${configFile}
      '';
    };

    # Systemd user service - auto-starts with graphical session
    systemd.user.services.ht32paneld = {
      Unit = {
        Description = "HT32 Panel Daemon";
        After = [ "graphical-session-pre.target" ];
        PartOf = [ "graphical-session.target" ];
      };

      Service = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/ht32paneld ${configFile}";
        Restart = "on-failure";
        RestartSec = 5;

        # State directory for persistent data (face selection, etc.)
        # For user services, this creates ~/.local/state/ht32-panel
        StateDirectory = "ht32-panel";
        StateDirectoryMode = "0750";

        # Hardening (user service compatible)
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        PrivateTmp = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectKernelLogs = true;
        ProtectControlGroups = true;
        ProtectClock = true;
        ProtectHostname = true;
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
        SystemCallArchitectures = "native";
      };

      Install = {
        WantedBy = [ "graphical-session.target" ];
      };
    };

    # Enable the service to auto-start
    systemd.user.startServices = "sd-switch";

    # Applet autostart
    xdg.configFile."autostart/ht32-panel-applet.desktop" = lib.mkIf (cfg.applet.enable && cfg.applet.autostart && cfg.applet.package != null) {
      text = ''
        [Desktop Entry]
        Type=Application
        Name=HT32 Panel Applet
        Comment=System tray applet for HT32 Panel control
        Exec=${cfg.applet.package}/bin/ht32-panel-applet
        Icon=display-brightness-symbolic
        Categories=System;Monitor;
        StartupNotify=false
        X-GNOME-Autostart-enabled=true
      '';
    };
  };

  meta.maintainers = [ ];
}
