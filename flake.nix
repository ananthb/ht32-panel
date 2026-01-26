{
  description = "HT32 Panel - Mini PC Display & LED Control";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devenv = {
      url = "github:cachix/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, devenv }@inputs:
    let
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      version = cargoToml.workspace.package.version;
    in
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };

        nativeBuildInputs = with pkgs; [
          pkg-config
          cmake
        ];

        buildInputs = with pkgs; [
          hidapi
          libusb1
          udev
          systemd
          dbus
        ];

        cargoArgs = {
          pname = "ht32-panel";
          inherit version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          inherit nativeBuildInputs buildInputs;
          cargoTestFlags = [ "--workspace" "--" "--skip" "test_device_open" ];

          meta = with pkgs.lib; {
            description = "HT32 Panel - Mini PC Display & LED Control";
            homepage = "https://github.com/ananthb/ht32-panel";
            license = licenses.agpl3Plus;
            platforms = [ "x86_64-linux" ];
          };
        };

      in {
        packages = {
          default = pkgs.rustPlatform.buildRustPackage (cargoArgs // {
            postInstall = ''
              mkdir -p $out/share/ht32-panel
              cp -r config $out/share/ht32-panel/
              cp -r themes $out/share/ht32-panel/
            '';
          });

          ht32paneld = pkgs.rustPlatform.buildRustPackage (cargoArgs // {
            pname = "ht32paneld";
            cargoBuildFlags = [ "-p" "ht32-panel-daemon" ];
            postInstall = ''
              mkdir -p $out/share/ht32-panel
              cp -r config $out/share/ht32-panel/
              cp -r themes $out/share/ht32-panel/
            '';
          });

          ht32panelctl = pkgs.rustPlatform.buildRustPackage (cargoArgs // {
            pname = "ht32panelctl";
            cargoBuildFlags = [ "-p" "ht32-panel-cli" ];
          });

          ht32-panel-applet = pkgs.rustPlatform.buildRustPackage (cargoArgs // {
            pname = "ht32-panel-applet";
            cargoBuildFlags = [ "-p" "ht32-panel-applet" ];
            buildInputs = buildInputs ++ (with pkgs; [
              glib
              gtk3
              libappindicator-gtk3
            ]);
          });
        };

        devShells.default = devenv.lib.mkShell {
          inherit inputs pkgs;
          modules = [ ./devenv.nix ];
        };
      }
    ) // {
      # NixOS modules (system-level service)
      nixosModules.default = { config, lib, pkgs, ... }: {
        imports = [ ./nix/module.nix ];
        config = lib.mkIf config.services.ht32-panel.enable {
          services.ht32-panel.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.default;
          services.ht32-panel.applet.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.ht32-panel-applet;
        };
      };
      nixosModules.ht32-panel = self.nixosModules.default;

      # Standalone udev rules module (for use with Home Manager)
      # Import this in your NixOS config when using the Home Manager module
      nixosModules.udevRules = { config, lib, ... }:
        let
          cfg = config.services.ht32-panel.udevRules;
        in {
          options.services.ht32-panel.udevRules = {
            enable = lib.mkEnableOption "udev rules for HT32 Panel hardware access";

            group = lib.mkOption {
              type = lib.types.str;
              default = "users";
              description = "Group to grant access to hardware devices.";
            };
          };

          config = lib.mkIf cfg.enable {
            services.udev.extraRules = ''
              # HT32 Panel LCD (VID:PID 04D9:FD01)
              SUBSYSTEM=="usb", ATTR{idVendor}=="04d9", ATTR{idProduct}=="fd01", MODE="0660", GROUP="${cfg.group}"
              SUBSYSTEM=="hidraw", ATTRS{idVendor}=="04d9", ATTRS{idProduct}=="fd01", MODE="0660", GROUP="${cfg.group}"

              # CH340 serial adapter for LED strip
              SUBSYSTEM=="tty", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", MODE="0660", GROUP="${cfg.group}", SYMLINK+="ht32-led"
            '';
          };
        };

      # Home Manager modules (user-level service)
      homeManagerModules.default = { config, lib, pkgs, osConfig ? null, ... }: {
        imports = [ ./nix/home-module.nix ];
        config = lib.mkIf config.services.ht32-panel.enable {
          services.ht32-panel.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.default;
          services.ht32-panel.cli.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.ht32panelctl;
          services.ht32-panel.applet.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.ht32-panel-applet;
        };
      };
      homeManagerModules.ht32-panel = self.homeManagerModules.default;

      overlays.default = final: prev: {
        ht32-panel = self.packages.${prev.system}.default;
        ht32paneld = self.packages.${prev.system}.ht32paneld;
        ht32panelctl = self.packages.${prev.system}.ht32panelctl;
        ht32-panel-applet = self.packages.${prev.system}.ht32-panel-applet;
      };
    };
}
