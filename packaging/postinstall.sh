#!/bin/sh
# Post-install for the ht32-panel (daemon) package.
set -e

# Make the new unit visible to systemd.
systemctl daemon-reload >/dev/null 2>&1 || true

# Apply the D-Bus system policy now, so the daemon can own its bus name
# without requiring a reboot. Support both dbus-daemon and dbus-broker.
systemctl reload dbus >/dev/null 2>&1 \
  || systemctl reload dbus-broker >/dev/null 2>&1 \
  || true

# Apply the udev rule for LCD/LED device permissions.
udevadm control --reload-rules >/dev/null 2>&1 || true
udevadm trigger >/dev/null 2>&1 || true

# Headless setup is admin-driven: the package ships config.toml.example, and
# ht32-panel.service expects /etc/ht32-panel/config.toml. Do not auto-enable.
echo "ht32-panel: create /etc/ht32-panel/config.toml (copy config.toml.example)," >&2
echo "            then run: systemctl enable --now ht32-panel.service" >&2

exit 0
