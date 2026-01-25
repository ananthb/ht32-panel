# HT32 Panel

Front-panel display and LED control for mini PCs with HT32-based LCD and RGB LEDs (AceMagic S1S, Skullsaints Agni, etc.).

## Features

- **Daemon** (`ht32paneld`): D-Bus service with HTMX web UI
- **CLI** (`ht32panelctl`): D-Bus client for daemon control
- **Applet**: System tray for GNOME/KDE

## Hardware

| Component | Interface | Details |
|-----------|-----------|---------|
| LCD Display | USB HID | VID:PID 04D9:FD01, 320x170 RGB565 |
| LED Strip | Serial | CH340, 10000 baud |

## Installation

### Flatpak

```bash
flatpak install ht32-panel-*.flatpak
flatpak run org.ht32panel.Daemon
```

### AppImage

```bash
chmod +x ht32-panel-*.AppImage
./ht32-panel-*.AppImage
```

### NixOS

```nix
{
  inputs.ht32-panel.url = "github:ananthb/ht32-panel";
}
```

```nix
{ inputs, ... }:
{
  imports = [ inputs.ht32-panel.nixosModules.default ];

  services.ht32-panel = {
    enable = true;
    openFirewall = true;
    led.theme = 2;  # breathing
  };
}
```

### Nix

```bash
nix run github:ananthb/ht32-panel
```

### From Source

```bash
git clone https://github.com/ananthb/ht32-panel
cd ht32-panel
cargo build --release
```

## Usage

```bash
# Start daemon (web UI at http://localhost:8686)
ht32paneld config/default.toml

# CLI (requires daemon)
ht32panelctl lcd orientation landscape
ht32panelctl led set rainbow --intensity 3 --speed 3
```

## D-Bus

The daemon exposes `org.ht32panel.Daemon1` on the session bus.

## Acknowledgement

Thanks to the research and source code from https://github.com/tjaworski/AceMagic-S1-LED-TFT-Linux.
This project was only possible because of that one.

## License

GPL-3.0
