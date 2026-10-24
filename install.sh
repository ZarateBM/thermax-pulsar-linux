#!/usr/bin/env bash
set -e

echo "=========================================================="
echo " Installing THERMAX 6 PULSAR Linux Driver & Service"
echo "=========================================================="

# Check root permissions
if [ "$EUID" -ne 0 ]; then
    echo "[!] Please run this installer with sudo: sudo ./install.sh"
    exit 1
fi

# Build release binary if not built yet
if [ ! -f "target/release/thermax-pulsar" ]; then
    echo "[*] Building release binary with Cargo..."
    cargo build --release
fi

echo "[*] Installing binary to /usr/local/bin/thermax-pulsar..."
install -m 755 target/release/thermax-pulsar /usr/local/bin/thermax-pulsar

echo "[*] Installing configuration..."
mkdir -p /etc/thermax-pulsar
if [ ! -f /etc/thermax-pulsar/config.toml ]; then
    install -m 644 config.toml /etc/thermax-pulsar/config.toml
    echo "    Created default config at /etc/thermax-pulsar/config.toml"
else
    echo "    Existing config found at /etc/thermax-pulsar/config.toml (keeping existing)."
fi

echo "[*] Installing udev rules..."
install -m 644 udev/99-thermax-pulsar.rules /etc/udev/rules.d/99-thermax-pulsar.rules
udevadm control --reload-rules
udevadm trigger

echo "[*] Installing systemd service..."
install -m 644 systemd/thermax-pulsar.service /etc/systemd/system/thermax-pulsar.service
systemctl daemon-reload
systemctl enable --now thermax-pulsar.service

echo ""
echo "=========================================================="
echo " [OK] Installation complete!"
echo " Service status:"
systemctl status thermax-pulsar.service --no-pager || true
echo "=========================================================="
echo " - To change settings (CPU/GPU mode, interval, unit):"
echo "     sudo nano /etc/thermax-pulsar/config.toml"
echo "     sudo systemctl restart thermax-pulsar"
echo " - To check logs:"
echo "     journalctl -u thermax-pulsar -f"
echo "=========================================================="
