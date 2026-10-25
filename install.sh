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

INSTALL_TYPE=""
for arg in "$@"; do
    case "$arg" in
        --python) INSTALL_TYPE="python" ;;
        --rust)   INSTALL_TYPE="rust" ;;
    esac
done

# 2. Selección del método si no fue especificado por parámetro
if [ -z "$INSTALL_TYPE" ]; then
    echo ""
    echo "Selecciona el método de instalación:"
    echo "  1) Versión Python (⚡ Recomendado: Instalación instantánea, 0 descargas, no necesita Rust)"
    echo "  2) Versión Rust   (🦀 Descarga e instala Rust automáticamente y compila el binario nativo)"
    echo ""
    read -p "Opción [1/2] (por defecto 1): " choice
    case "$choice" in
        2) INSTALL_TYPE="rust" ;;
        *) INSTALL_TYPE="python" ;;
    esac
fi

echo ""
echo "[*] Preparando instalación (Modo: ${INSTALL_TYPE^^})..."

# 3. Construir / preparar el ejecutable
if [ "$INSTALL_TYPE" = "rust" ]; then
    # Ubicar cargo en el HOME del usuario sudo o del sistema
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

    install -m 755 target/release/thermax-pulsar /usr/local/bin/thermax-pulsar

else
    # Versión Python directa
    echo "[*] Instalando driver Python en /usr/local/bin/thermax-pulsar..."
    install -m 755 thermax_pulsar.py /usr/local/bin/thermax-pulsar
fi

# 4. Instalar configuración
echo "[*] Configurando /etc/thermax-pulsar/config.toml..."
mkdir -p /etc/thermax-pulsar
if [ ! -f /etc/thermax-pulsar/config.toml ]; then
    install -m 644 config.toml /etc/thermax-pulsar/config.toml
    echo "    -> Archivo de configuración creado."
else
    echo "    -> Configuración existente detectada (se mantiene la actual)."
fi

# 5. Instalar reglas udev
echo "[*] Instalando reglas de permisos USB (udev)..."
install -m 644 udev/99-thermax-pulsar.rules /etc/udev/rules.d/99-thermax-pulsar.rules
udevadm control --reload-rules
udevadm trigger

# 6. Instalar y activar servicio systemd
echo "[*] Instalando servicio de inicio automático (systemd)..."
install -m 644 systemd/thermax-pulsar.service /etc/systemd/system/thermax-pulsar.service
systemctl daemon-reload
systemctl enable thermax-pulsar.service
systemctl restart thermax-pulsar.service

echo ""
echo "=========================================================="
echo " 🎉 [OK] ¡Instalación/Actualización completada exitosamente!"
echo " Estado del servicio:"
systemctl status thermax-pulsar.service --no-pager || true
echo "=========================================================="
echo " - El display se iniciará automáticamente cada vez que prendas tu PC."
if [ "$INSTALL_TYPE" = "rust" ]; then
echo " - 🌐 Panel Web de Control y Monitoreo:"
echo "     thermax-pulsar web"
echo "     (Luego abre http://localhost:9876 en tu navegador)"
fi
echo " - Para cambiar ajustes manualmente:"
echo "     sudo nano /etc/thermax-pulsar/config.toml"
echo "     sudo systemctl restart thermax-pulsar"
echo " - Para ver los logs en tiempo real:"
echo "     journalctl -u thermax-pulsar -f"
echo " - Para desinstalar en cualquier momento:"
echo "     sudo ./uninstall.sh"
echo "=========================================================="
