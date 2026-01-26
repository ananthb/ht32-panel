{ config, lib, pkgs, ... }:

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
    lcd = {
      device = cfg.lcd.device;
    };
    led = {
      device = cfg.led.device;
      theme = cfg.led.theme;
      intensity = cfg.led.intensity;
      speed = cfg.led.speed;
    };
    canvas = {
      width = cfg.canvas.width;
      height = cfg.canvas.height;
    };
  } // cfg.extraSettings);
in
{
  options.services.ht32-panel = {
    enable = lib.mkEnableOption "HT32 Panel daemon for LCD and LED control";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The ht32-panel package to use.";
    };

    udevRules = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Install udev rules for HT32 Panel hardware access.
          Enable this even if using the Home Manager module for the daemon,
          to grant your user access to the hardware devices.
        '';
      };

      group = lib.mkOption {
        type = lib.types.str;
        default = if cfg.enable then cfg.group else "plugdev";
        defaultText = lib.literalExpression ''if cfg.enable then cfg.group else "plugdev"'';
        description = "Group to grant access to hardware devices.";
      };
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "ht32-panel";
      description = "User account under which the daemon runs.";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "ht32-panel";
      description = "Group under which the daemon runs.";
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
        default = "system";
        description = ''
          Which D-Bus bus to use.
          - "system": Use the system bus (recommended for system services).
          - "session": Use the session bus (for user services).
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

    lcd = {
      device = lib.mkOption {
        type = lib.types.str;
        default = "auto";
        description = "LCD device path or 'auto' for auto-detection.";
      };
    };

    led = {
      device = lib.mkOption {
        type = lib.types.str;
        default = "/dev/ttyUSB0";
        description = "Serial port path for LED controller.";
      };

      theme = lib.mkOption {
        type = lib.types.ints.between 1 5;
        default = 2;
        description = "LED theme (1=rainbow, 2=breathing, 3=colors, 4=off, 5=auto).";
      };

      intensity = lib.mkOption {
        type = lib.types.ints.between 1 5;
        default = 3;
        description = "LED intensity (1-5).";
      };

      speed = lib.mkOption {
        type = lib.types.ints.between 1 5;
        default = 3;
        description = "LED animation speed (1-5).";
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

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Whether to open the firewall port for the web interface.";
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
  };

  config = lib.mkMerge [
    # Udev rules - can be enabled independently for Home Manager users
    (lib.mkIf cfg.udevRules.enable {
      # Udev rules for USB HID access
      services.udev.extraRules = ''
        # HT32 Panel LCD (VID:PID 04D9:FD01)
        SUBSYSTEM=="usb", ATTR{idVendor}=="04d9", ATTR{idProduct}=="fd01", MODE="0660", GROUP="${cfg.udevRules.group}"
        SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04d9", ATTRS{idProduct}=="fd01", MODE="0660", GROUP="${cfg.udevRules.group}"

        # CH340 serial adapter for LED strip
        SUBSYSTEM=="tty", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", MODE="0660", GROUP="${cfg.udevRules.group}", SYMLINK+="ht32-led"
      '';
    })

    # Full service configuration
    (lib.mkIf cfg.enable {
      # Create user and group
      users.users.${cfg.user} = lib.mkIf (cfg.user == "ht32-panel") {
        isSystemUser = true;
        group = cfg.group;
        description = "HT32 Panel daemon user";
        extraGroups = [ "dialout" "plugdev" ];
      };

      users.groups.${cfg.group} = lib.mkIf (cfg.group == "ht32-panel") { };

      # D-Bus policy files
      services.dbus.packages = [
        (pkgs.writeTextFile {
          name = "ht32-panel-dbus";
          destination = "/share/dbus-1/${if cfg.dbus.bus == "session" then "services" else "system.d"}/org.ht32panel.Daemon.${if cfg.dbus.bus == "session" then "service" else "conf"}";
          text = if cfg.dbus.bus == "session" then ''
            [D-BUS Service]
            Name=org.ht32panel.Daemon
            Exec=${cfg.package}/bin/ht32paneld ${configFile}
            User=${cfg.user}
          '' else ''
            <!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
              "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
            <busconfig>
              <!-- Allow ht32-panel user to own the service name -->
              <policy user="${cfg.user}">
                <allow own="org.ht32panel.Daemon"/>
                <allow send_destination="org.ht32panel.Daemon"/>
                <allow receive_sender="org.ht32panel.Daemon"/>
              </policy>

              <!-- Allow anyone to call methods on the interface -->
              <policy context="default">
                <allow send_destination="org.ht32panel.Daemon"/>
                <allow receive_sender="org.ht32panel.Daemon"/>
              </policy>
            </busconfig>
          '';
        })
      ];

      # Systemd service
      systemd.services.ht32paneld = {
        description = "HT32 Panel Daemon";
        wantedBy = [ "multi-user.target" ];
        after = [ "network.target" "dbus.service" ];
        requires = [ "dbus.service" ];

        serviceConfig = {
          Type = "simple";
          User = cfg.user;
          Group = cfg.group;
          ExecStart = "${cfg.package}/bin/ht32paneld ${configFile}";
          Restart = "on-failure";
          RestartSec = 5;

          # Directories managed by systemd
          StateDirectory = "ht32-panel";
          ConfigurationDirectory = "ht32-panel";

          # Hardening
          NoNewPrivileges = true;
          ProtectSystem = "strict";
          ProtectHome = true;
          PrivateTmp = true;
          ProtectKernelTunables = true;
          ProtectKernelModules = true;
          ProtectKernelLogs = true;
          ProtectControlGroups = true;
          ProtectClock = true;
          ProtectHostname = true;
          ProtectProc = "invisible";
          ProcSubset = "pid";
          RestrictAddressFamilies = [ "AF_UNIX" "AF_INET" "AF_INET6" ];
          RestrictNamespaces = true;
          RestrictRealtime = true;
          RestrictSUIDSGID = true;
          LockPersonality = true;
          MemoryDenyWriteExecute = true;
          SystemCallArchitectures = "native";
          SystemCallFilter = [ "@system-service" "~@privileged" "~@resources" ];
          CapabilityBoundingSet = "";

          # Device access for LCD (hidraw) and LED (serial)
          DevicePolicy = "closed";
          DeviceAllow = [
            "/dev/hidraw* rw"
            "/dev/ttyUSB* rw"
            "/dev/ttyACM* rw"
            "char-usb_device rw"
          ];

          # Supplementary groups for device access
          SupplementaryGroups = [ "dialout" ];
        };
      };

      # Open firewall if requested (only if web server is enabled)
      networking.firewall = lib.mkIf (cfg.openFirewall && cfg.web.enable) {
        allowedTCPPorts = [
          (lib.toInt (lib.last (lib.splitString ":" cfg.web.listen)))
        ];
      };

      # Add package to system packages for CLI access
      environment.systemPackages = [ cfg.package ]
        ++ lib.optional (cfg.applet.enable && cfg.applet.package != null) cfg.applet.package;

      # Applet autostart desktop entry
      environment.etc = lib.mkIf (cfg.applet.enable && cfg.applet.autostart && cfg.applet.package != null) {
        "xdg/autostart/ht32-panel-applet.desktop".text = ''
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
    })
  ];

  meta.maintainers = [ ];
}
