{ config, lib, pkgs, ... }:

let
  cfg = config.services.ht32-panel;
  settingsFormat = pkgs.formats.toml { };
in
{
  options.services.ht32-panel = {
    enable = lib.mkEnableOption "HT32 Panel daemon for LCD and LED control";

    package = lib.mkPackageOption pkgs "ht32-panel" {
      default = null;
      description = "The ht32-panel package to use.";
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

    listen = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0:8686";
      description = "Address and port for the web interface.";
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

    configDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/ht32-panel";
      description = "Directory for configuration and theme files.";
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

      package = lib.mkPackageOption pkgs "ht32-panel-applet" {
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

  config = lib.mkIf cfg.enable {
    # Create user and group
    users.users.${cfg.user} = lib.mkIf (cfg.user == "ht32-panel") {
      isSystemUser = true;
      group = cfg.group;
      description = "HT32 Panel daemon user";
      extraGroups = [ "dialout" "plugdev" ];
    };

    users.groups.${cfg.group} = lib.mkIf (cfg.group == "ht32-panel") { };

    # Udev rules for USB HID access
    services.udev.extraRules = ''
      # HT32 Panel LCD (VID:PID 04D9:FD01)
      SUBSYSTEM=="usb", ATTR{idVendor}=="04d9", ATTR{idProduct}=="fd01", MODE="0666", GROUP="${cfg.group}"
      SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04d9", ATTRS{idProduct}=="fd01", MODE="0666", GROUP="${cfg.group}"

      # CH340 serial adapter for LED strip
      SUBSYSTEM=="tty", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", MODE="0666", GROUP="${cfg.group}", SYMLINK+="ht32-led"
    '';

    # D-Bus service file for on-demand activation
    services.dbus.packages = lib.mkIf cfg.enable [
      (pkgs.writeTextFile {
        name = "ht32-panel-dbus";
        destination = "/share/dbus-1/services/org.ht32panel.Daemon.service";
        text = ''
          [D-BUS Service]
          Name=org.ht32panel.Daemon
          Exec=${cfg.package}/bin/ht32paneld ${cfg.configDir}/config.toml
          User=${cfg.user}
        '';
      })
    ];

    # Systemd service
    systemd.services.ht32-panel = {
      description = "HT32 Panel Daemon";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" "dbus.service" ];
      requires = [ "dbus.service" ];

      environment = {
        DBUS_SESSION_BUS_ADDRESS = "unix:path=/run/user/\${UID}/bus";
      };

      serviceConfig = {
        Type = "simple";
        User = cfg.user;
        Group = cfg.group;
        ExecStart = "${cfg.package}/bin/ht32paneld ${cfg.configDir}/config.toml";
        Restart = "on-failure";
        RestartSec = 5;

        # Hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        ReadWritePaths = [ cfg.configDir ];

        # Allow access to USB devices
        DeviceAllow = [
          "/dev/hidraw* rw"
          "/dev/ttyUSB* rw"
          "/dev/ttyACM* rw"
        ];
      };

      preStart = ''
        # Ensure config directory exists
        mkdir -p ${cfg.configDir}/themes

        # Generate config file
        cat > ${cfg.configDir}/config.toml << EOF
        listen = "${cfg.listen}"
        theme = "${cfg.theme}"
        poll = ${toString cfg.poll}
        refresh = ${toString cfg.refresh}
        heartbeat = ${toString cfg.heartbeat}

        [lcd]
        device = "${cfg.lcd.device}"

        [led]
        device = "${cfg.led.device}"
        theme = ${toString cfg.led.theme}
        intensity = ${toString cfg.led.intensity}
        speed = ${toString cfg.led.speed}

        [canvas]
        width = ${toString cfg.canvas.width}
        height = ${toString cfg.canvas.height}
        EOF

        # Copy default theme if not present
        if [ ! -f ${cfg.configDir}/themes/default.toml ]; then
          cp ${cfg.package}/share/ht32-panel/themes/default.toml ${cfg.configDir}/themes/
        fi
      '';
    };

    # Open firewall if requested
    networking.firewall = lib.mkIf cfg.openFirewall {
      allowedTCPPorts = [
        (lib.toInt (lib.last (lib.splitString ":" cfg.listen)))
      ];
    };

    # Add package to system packages for CLI access
    environment.systemPackages = [ cfg.package ]
      ++ lib.optional cfg.applet.enable cfg.applet.package;

    # Applet autostart desktop entry
    xdg.autostart.entries = lib.mkIf (cfg.applet.enable && cfg.applet.autostart) {
      "ht32-panel-applet" = {
        name = "HT32 Panel Applet";
        exec = "${cfg.applet.package}/bin/ht32-panel-applet";
        icon = "display-brightness-symbolic";
        comment = "System tray applet for HT32 Panel control";
        categories = [ "System" "Monitor" ];
        startupNotify = false;
      };
    };

    # Alternative: Create desktop file directly for broader compatibility
    environment.etc = lib.mkIf (cfg.applet.enable && cfg.applet.autostart) {
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
  };

  meta.maintainers = [ ];
}
