#!/usr/bin/env bash
set -e

echo "=========================================================="
echo " Instalador THERMAX 6 PULSAR - Driver, App & Servicio Linux"
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

# 3. Instalar ejecutable de ventana GUI nativa
echo "[*] Instalando ejecutable de ventana gráfica (GTK)..."
install -m 755 assets/thermax-pulsar-window /usr/local/bin/thermax-pulsar-window

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

# 6. Instalar iconos oficiales del sistema
echo "[*] Instalando iconos de la aplicación..."
mkdir -p /usr/share/icons/hicolor/scalable/apps
install -m 644 assets/icons/thermax-pulsar.svg /usr/share/icons/hicolor/scalable/apps/thermax-pulsar.svg

for size in 512 256 128 64 48 32; do
    if [ -f "assets/icons/hicolor/${size}x${size}/apps/thermax-pulsar.png" ]; then
        mkdir -p "/usr/share/icons/hicolor/${size}x${size}/apps"
        install -m 644 "assets/icons/hicolor/${size}x${size}/apps/thermax-pulsar.png" "/usr/share/icons/hicolor/${size}x${size}/apps/thermax-pulsar.png"
    fi
done

# Copia de compatibilidad en pixmaps
mkdir -p /usr/share/pixmaps
if [ -f "assets/icons/hicolor/256x256/apps/thermax-pulsar.png" ]; then
    install -m 644 assets/icons/hicolor/256x256/apps/thermax-pulsar.png /usr/share/pixmaps/thermax-pulsar.png
fi

gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true

# 7. Instalar lanzador de escritorio (.desktop) en el cajón de aplicaciones
echo "[*] Registrando aplicación de escritorio en el cajón de aplicaciones de Linux..."
install -m 644 assets/thermax-pulsar.desktop /usr/share/applications/thermax-pulsar.desktop
update-desktop-database /usr/share/applications 2>/dev/null || true

# 8. Instalar y activar servicio systemd
echo "[*] Instalando servicio de inicio automático (systemd)..."
install -m 644 systemd/thermax-pulsar.service /etc/systemd/system/thermax-pulsar.service
systemctl daemon-reload
systemctl enable thermax-pulsar.service
systemctl restart thermax-pulsar.service

echo ""
echo "=========================================================="
echo " 🎉 [OK] ¡Instalación completada exitosamente!"
echo " Estado del servicio en segundo plano:"
systemctl status thermax-pulsar.service --no-pager || true
echo "=========================================================="
echo " - 🖥️ Aplicación de Escritorio:"
echo "     Ya está disponible en tu Cajón de Aplicaciones (menú de inicio):"
echo "     Busca 'THERMAX 6 PULSAR' y ábrela con un solo clic con su propio icono."
echo ""
echo " - También puedes iniciar la app desde la terminal con:"
echo "     thermax-pulsar app"
echo ""
echo " - El display se mantendrá activo siempre en segundo plano."
echo " - Para ver los logs del servicio:"
echo "     journalctl -u thermax-pulsar -f"
echo " - Para desinstalar en cualquier momento:"
echo "     sudo ./uninstall.sh"
echo "=========================================================="
