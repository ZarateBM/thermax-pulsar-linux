#!/usr/bin/env bash
set -e

echo "=========================================================="
echo " Instalador THERMAX 6 PULSAR - Driver & Servicio Linux"
echo "=========================================================="

# 1. Verificar permisos de root
if [ "$EUID" -ne 0 ]; then
    echo "[!] Por favor ejecuta este instalador con sudo:"
    echo "    sudo ./install.sh"
    exit 1
fi

echo ""
echo "[*] Preparando instalación (Modo: RUST Nativo)..."

# 2. Construir / preparar el ejecutable en Rust
USER_CARGO_DIR=""
if [ -n "$SUDO_USER" ] && [ -x "$(eval echo ~$SUDO_USER)/.cargo/bin/cargo" ]; then
    USER_CARGO_DIR="$(eval echo ~$SUDO_USER)/.cargo/bin"
    export PATH="$USER_CARGO_DIR:$PATH"
fi

if command -v cargo &> /dev/null; then
    echo "[*] Compilando binario optimizado en Rust..."
    if [ -n "$SUDO_USER" ] && [ -n "$USER_CARGO_DIR" ]; then
        sudo -u "$SUDO_USER" env "PATH=$USER_CARGO_DIR:$PATH" cargo build --release
    else
        cargo build --release
    fi
elif [ -f "target/release/thermax-pulsar" ]; then
    echo "[*] Usando binario precompilado detectado en target/release/thermax-pulsar."
else
    echo "[*] Instalando dependencias de compilación del sistema (apt)..."
    apt-get update -qq
    apt-get install -y -qq build-essential pkg-config libudev-dev curl

    echo "[*] Instalando compilador Rust automáticamente..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    export PATH="$HOME/.cargo/bin:$PATH"
    rustup default stable 2>/dev/null || true

    echo "[*] Compilando binario optimizado en Rust..."
    cargo build --release
fi

if [ ! -f "target/release/thermax-pulsar" ]; then
    echo "[!] Error: No se encontró el binario compilado en target/release/thermax-pulsar"
    exit 1
fi

echo "[*] Instalando binario en /usr/local/bin/thermax-pulsar..."
install -m 755 target/release/thermax-pulsar /usr/local/bin/thermax-pulsar

# 3. Instalar configuración
echo "[*] Configurando /etc/thermax-pulsar/config.toml..."
mkdir -p /etc/thermax-pulsar
if [ ! -f /etc/thermax-pulsar/config.toml ]; then
    install -m 644 config.toml /etc/thermax-pulsar/config.toml
    echo "    -> Archivo de configuración creado."
else
    echo "    -> Configuración existente detectada (se mantiene la actual)."
fi

# 4. Instalar reglas udev
echo "[*] Instalando reglas de permisos USB (udev)..."
install -m 644 udev/99-thermax-pulsar.rules /etc/udev/rules.d/99-thermax-pulsar.rules
udevadm control --reload-rules
udevadm trigger

# 5. Instalar y activar servicio systemd
echo "[*] Instalando servicio de inicio automático (systemd)..."
install -m 644 systemd/thermax-pulsar.service /etc/systemd/system/thermax-pulsar.service
systemctl daemon-reload
systemctl enable thermax-pulsar.service
systemctl restart thermax-pulsar.service

echo ""
echo "=========================================================="
echo " 🎉 [OK] ¡Instalación completada exitosamente!"
echo " Estado del servicio:"
systemctl status thermax-pulsar.service --no-pager || true
echo "=========================================================="
echo " - El display se iniciará automáticamente cada vez que prendas tu PC."
echo " - 🌐 Panel Web de Control y Monitoreo:"
echo "     thermax-pulsar web"
echo "     (Luego abre http://localhost:9876 en tu navegador)"
echo " - Para cambiar ajustes manualmente:"
echo "     sudo nano /etc/thermax-pulsar/config.toml"
echo "     sudo systemctl restart thermax-pulsar"
echo " - Para ver los logs en tiempo real:"
echo "     journalctl -u thermax-pulsar -f"
echo " - Para desinstalar en cualquier momento:"
echo "     sudo ./uninstall.sh"
echo "=========================================================="
