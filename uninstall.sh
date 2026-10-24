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
systemctl daemon-reload
udevadm control --reload-rules

echo "[?] Do you want to remove configuration directory (/etc/thermax-pulsar)? [y/N]"
read -r response
if [[ "$response" =~ ^([yY][eE][sS]|[yY])$ ]]; then
    rm -rf /etc/thermax-pulsar
    echo "Configuration removed."
fi

echo "[OK] Uninstallation complete."
