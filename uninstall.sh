#!/usr/bin/env bash
set -e

if [ "$EUID" -ne 0 ]; then
    echo "[!] Please run with sudo: sudo ./uninstall.sh"
    exit 1
fi

echo "[*] Stopping and disabling systemd service..."
systemctl stop thermax-pulsar.service 2>/dev/null || true
systemctl disable thermax-pulsar.service 2>/dev/null || true

echo "[*] Removing installed files..."
rm -f /etc/systemd/system/thermax-pulsar.service
rm -f /etc/udev/rules.d/99-thermax-pulsar.rules
rm -f /usr/local/bin/thermax-pulsar
rm -f /usr/local/bin/thermax-pulsar-window
rm -f /usr/share/applications/thermax-pulsar.desktop
rm -f /usr/share/icons/hicolor/scalable/apps/thermax-pulsar.svg
rm -f /usr/share/icons/hicolor/*/apps/thermax-pulsar.png
rm -f /usr/share/pixmaps/thermax-pulsar.png

# Remove user-local desktop entry (kept by some DEs in the app hub)
if [ -n "$SUDO_USER" ]; then
    USER_HOME=$(eval echo ~"$SUDO_USER")
    rm -f "$USER_HOME/.local/share/applications/thermax-pulsar.desktop"
    sudo -u "$SUDO_USER" update-desktop-database "$USER_HOME/.local/share/applications" 2>/dev/null || true
fi

systemctl daemon-reload
udevadm control --reload-rules
update-desktop-database /usr/share/applications 2>/dev/null || true
gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true

echo "[?] Do you want to remove configuration directory (/etc/thermax-pulsar)? [y/N]"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    rm -rf /etc/thermax-pulsar
    echo "Configuration removed."
fi

echo "[OK] Uninstallation complete."
