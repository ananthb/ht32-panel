{
  description = "HT32 Panel - Mini PC Display & LED Control";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, git-hooks, rust-overlay }:
    let
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      version = cargoToml.workspace.package.version;
    in
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };

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

        appletBuildInputs = buildInputs ++ (with pkgs; [
          glib
          gtk3
          libappindicator-gtk3
        ]);

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

        pre-commit = git-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            check-json.enable = true;
            check-merge-conflicts.enable = true;
            check-toml.enable = true;
            check-yaml.enable = true;
            clippy = {
              enable = true;
              entry = let
                depsPath = pkgs.lib.makeBinPath ([ rustToolchain ] ++ nativeBuildInputs);
                pkgConfigPath = pkgs.lib.makeSearchPath "lib/pkgconfig"
                  (map (p: if p ? dev then p.dev else p) appletBuildInputs);
              in toString (pkgs.writeShellScript "clippy-hook" ''
                export PATH="${depsPath}''${PATH:+:$PATH}"
                export PKG_CONFIG_PATH="${pkgConfigPath}"
                cargo clippy --workspace --all-targets --offline -- -D warnings
              '');
              files = "\\.rs$";
              types = [ "file" ];
              pass_filenames = false;
            };
            detect-private-keys.enable = true;
            end-of-file-fixer.enable = true;
            rustfmt = {
              enable = true;
              packageOverrides.cargo = rustToolchain;
              packageOverrides.rustfmt = rustToolchain;
            };
            trim-trailing-whitespace.enable = true;
          };
        };

      in {
        packages = {
          default = pkgs.rustPlatform.buildRustPackage (cargoArgs // {
            postInstall = ''
              mkdir -p $out/share/ht32-panel
              cp -r config $out/share/ht32-panel/
            '';
          });

          ht32paneld = pkgs.rustPlatform.buildRustPackage (cargoArgs // {
            pname = "ht32paneld";
            cargoBuildFlags = [ "-p" "ht32-panel-daemon" ];
            postInstall = ''
              mkdir -p $out/share/ht32-panel
              cp -r config $out/share/ht32-panel/
            '';
          });

          ht32panelctl = pkgs.rustPlatform.buildRustPackage (cargoArgs // {
            pname = "ht32panelctl";
            cargoBuildFlags = [ "-p" "ht32-panel-cli" ];
          });

          ht32-panel-applet = pkgs.rustPlatform.buildRustPackage (cargoArgs // {
            pname = "ht32-panel-applet";
            cargoBuildFlags = [ "-p" "ht32-panel-applet" ];
            buildInputs = appletBuildInputs;
          });

          release-tarball = let
            pkg = self.packages.${system}.default;
            applet = self.packages.${system}.ht32-panel-applet;
          in pkgs.runCommand "ht32-panel-${version}-x86_64-linux.tar.gz" {
            nativeBuildInputs = [ pkgs.gzip pkgs.patchelf ];
          } ''
            mkdir -p dist/config
            cp ${pkg}/bin/ht32paneld dist/
            cp ${pkg}/bin/ht32panelctl dist/
            cp ${applet}/bin/ht32-panel-applet dist/
            chmod +w dist/ht32paneld dist/ht32panelctl dist/ht32-panel-applet
            patchelf --remove-rpath dist/ht32paneld
            patchelf --remove-rpath dist/ht32panelctl
            patchelf --remove-rpath dist/ht32-panel-applet
            cp -r ${pkg}/share/ht32-panel/config/* dist/config/
            tar -czvf $out -C dist .
          '';

          release-appimage = let
            pkg = self.packages.${system}.default;
            applet = self.packages.${system}.ht32-panel-applet;

            appimageRuntime = pkgs.fetchurl {
              url = "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64";
              hash = "sha256-L8qLRDySUQ8Ug6iD9gBhrQm0a5eLJjHIB82HOkfsJg0=";
            };

            libDeps = with pkgs; [
              hidapi
              libusb1
              udev
              systemd
              dbus
              glib
              gtk3
              libappindicator-gtk3
              pango
              cairo
              gdk-pixbuf
              atk
              harfbuzz
              fontconfig
              freetype
              libGL
              xorg.libX11
              xorg.libXcursor
              xorg.libXrandr
              xorg.libXi
              xorg.libXext
              xorg.libXrender
              xorg.libXfixes
              xorg.libXcomposite
              xorg.libXdamage
              xorg.libxcb
              libxkbcommon
              wayland
            ];
          in pkgs.runCommand "ht32-panel-${version}-x86_64.AppImage" {
            nativeBuildInputs = with pkgs; [ squashfsTools patchelf ];
          } ''
            # Create AppDir structure
            mkdir -p AppDir/usr/bin
            mkdir -p AppDir/usr/lib
            mkdir -p AppDir/usr/share/applications
            mkdir -p AppDir/usr/share/icons/hicolor/scalable/apps

            # Copy binaries and strip Nix RPATH
            cp ${pkg}/bin/ht32paneld AppDir/usr/bin/
            cp ${pkg}/bin/ht32panelctl AppDir/usr/bin/
            cp ${applet}/bin/ht32-panel-applet AppDir/usr/bin/
            chmod +w AppDir/usr/bin/*
            patchelf --remove-rpath AppDir/usr/bin/ht32paneld
            patchelf --remove-rpath AppDir/usr/bin/ht32panelctl
            patchelf --remove-rpath AppDir/usr/bin/ht32-panel-applet

            # Bundle shared libraries so the AppImage is self-contained.
            for dir in ${pkgs.lib.concatStringsSep " " (map (d: "${d}/lib") libDeps)}; do
              if [ -d "$dir" ]; then
                for so in "$dir"/*.so "$dir"/*.so.*; do
                  [ -e "$so" ] || continue
                  cp -n "$(readlink -f "$so")" "AppDir/usr/lib/$(basename "$so")" 2>/dev/null || true
                done
              fi
            done

            # Desktop file at root (required by AppImage spec)
            cp ${./packaging/org.ht32panel.Daemon.desktop} AppDir/ht32-panel.desktop
            cp ${./packaging/org.ht32panel.Daemon.desktop} AppDir/usr/share/applications/

            # Icon at root (required by AppImage spec)
            cp ${./packaging/org.ht32panel.Daemon.svg} AppDir/ht32-panel.svg
            cp ${./packaging/org.ht32panel.Daemon.svg} AppDir/usr/share/icons/hicolor/scalable/apps/org.ht32panel.Daemon.svg
            cp ${./packaging/org.ht32panel.Daemon.svg} AppDir/.DirIcon

            # Create AppRun launcher
            cat > AppDir/AppRun << 'APPRUN'
#!/bin/bash
set -e
SELF=$(readlink -f "$0")
APPDIR=''${SELF%/*}

export LD_LIBRARY_PATH="''${APPDIR}/usr/lib:''${LD_LIBRARY_PATH}"

# GTK/GLib settings
export GSETTINGS_SCHEMA_DIR="/usr/share/glib-2.0/schemas:''${GSETTINGS_SCHEMA_DIR}"
export GDK_PIXBUF_MODULE_FILE="/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"

exec "''${APPDIR}/usr/bin/ht32-panel-applet" "$@"
APPRUN
            chmod +x AppDir/AppRun

            # Create squashfs
            mksquashfs AppDir appimage.squashfs -root-owned -noappend -comp zstd -quiet -no-progress

            # Combine runtime + squashfs to create AppImage
            cat ${appimageRuntime} appimage.squashfs > $out
            chmod +x $out
          '';

          # Combined release with all artifacts for Garnix
          release = let
            tarball = self.packages.${system}.release-tarball;
            appimage = self.packages.${system}.release-appimage;
          in pkgs.runCommand "ht32-panel-${version}-release" {} ''
            mkdir -p $out
            cp ${tarball} $out/ht32-panel-${version}-x86_64-linux.tar.gz
            cp ${appimage} $out/ht32-panel-${version}-x86_64.AppImage
          '';
        };

        checks = {
          fmt = pkgs.runCommand "check-fmt" {
            nativeBuildInputs = [ rustToolchain ];
            src = self;
          } ''
            cd $src
            cargo fmt --all -- --check
            touch $out
          '';

          clippy = pkgs.rustPlatform.buildRustPackage (cargoArgs // {
            pname = "ht32-panel-clippy";
            nativeBuildInputs = nativeBuildInputs ++ [
              pkgs.clippy
              pkgs.rustPlatform.cargoSetupHook
            ];
            buildInputs = appletBuildInputs;
            buildPhase = ''
              runHook preBuild
              cargo clippy --workspace --all-targets --offline -- -D warnings
              runHook postBuild
            '';
            installPhase = "mkdir -p $out";
            doCheck = false;
          });

          tests = self.packages.${system}.default;
        };

        devShells.default = pkgs.mkShell {
          name = "ht32-panel-dev";

          packages = [
            rustToolchain
          ] ++ (with pkgs; [
            # Development tools
            cargo-nextest
            cargo-watch
            cargo-audit
            cargo-outdated
            just
            watchexec

            # Python (for flatpak-cargo-generator)
            (python3.withPackages (ps: [ ps.aiohttp ps.toml ]))
          ]) ++ nativeBuildInputs ++ appletBuildInputs;

          RUST_BACKTRACE = "1";
          RUST_LOG = "info";

          shellHook = ''
            ${pre-commit.shellHook}
            echo ""
            echo "HT32 Panel Development Environment"
            echo ""
            echo "Build:    cargo build --workspace"
            echo "Test:     cargo nextest run --workspace"
            echo "Lint:     cargo clippy --workspace --all-targets -- -D warnings"
            echo "Format:   cargo fmt --all"
            echo "Daemon:   cargo run -p ht32-panel-daemon -- config/default.toml"
            echo ""
          '';
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
