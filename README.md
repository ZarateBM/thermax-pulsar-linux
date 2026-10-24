# THERMAX 6 PULSAR - Linux Driver & Daemon (Rust)

Controlador ultraligero y demonio para el display digital del disipador por aire **THERMAX 6 PULSAR** (fabricante/software original: XYZ-WC-AQUA / `xyzpcgear.com`) en **Ubuntu / Linux**.

Creado mediante ingeniería inversa completa del binario original de Windows (`XYZ-WC-AQUA.exe`). Escrito en **Rust** con consumo mínimo de recursos (< 5 MB RAM, 0% CPU) y soporte nativo como servicio **systemd** para inicio automático al arrancar el sistema.

---

## Características

- **Display de Temperatura Seleccionable**: Puedes elegir si el display principal muestra:
  - `cpu`: Temperatura de la CPU.
  - `gpu`: Temperatura de la GPU.
  - `alternate`: Alterna automáticamente entre CPU y GPU cada N segundos.
  - `highest`: Muestra dinámicamente cuál de los dos componentes está más caliente.
- **Reloj y Fecha en Pantalla**: Sincroniza la hora y fecha del sistema con el display.
- **Soporte para °C y °F**: Configurable en Celsius o Fahrenheit.
- **Detección Automática de Hardware**:
  - CPU: Lee métricas nativas vía `/sys/class/hwmon` (drivers `coretemp`, `k10temp`, `zenpower`) y `sysinfo`.
  - GPU: Detección automática para NVIDIA (`nvidia-smi`), AMD (`/sys/class/drm/*/device/hwmon`) e Intel.
- **Inicio Automático con systemd**: Se inicia automáticamente en segundo plano al arrancar Ubuntu.
- **Sin necesidad de Root**: Incluye reglas `udev` para acceso seguro a `/dev/hidraw*`.
- **Mínimo Consumo**: Ejecutable estático en Rust con cero sobrecarga.

---

## Protocolo Descubierto (Ingeniería Inversa)

- **Vendor ID (VID)**: `0x5131`
- **Product ID (PID)**: `0x2007`
- **Interface**: USB HID (`/dev/hidraw*` en Linux)
- **Tamaño del Reporte**: 65 bytes (Report ID `0x00` + 64 bytes de payload)
- **Frecuencia de Envío**: Cada 500 ms - 1000 ms

### Estructura del Paquete HID (65 bytes)

| Offset | Nombre / Campo | Tipo | Descripción |
|---|---|---|---|
| `0` | Report ID | `u8` | Siempre `0x00` |
| `1` | Magic / Command | `u8` | Siempre `0x40` |
| `2` | **CPU Temp (Display Dial)** | `u8` | Temperatura en °C (si es °F: `temp \| 0x80`) |
| `3` | CPU Utilization | `u8` | Porcentaje de uso CPU (0 - 100) |
| `4 - 5` | CPU Frequency | `u16` (Big-Endian) | Frecuencia de la CPU en MHz |
| `6 - 7` | CPU Fan Speed | `u16` (Big-Endian) | Velocidad del ventilador en RPM |
| `8 - 9` | CPU Power | `u16` (Big-Endian) | Consumo CPU en Watts |
| `10` | **GPU Temp** | `u8` | Temperatura en °C (si es °F: `temp \| 0x80`) |
| `11` | GPU Utilization | `u8` | Porcentaje de uso GPU (0 - 100) |
| `12 - 13` | GPU Frequency | `u16` (Big-Endian) | Frecuencia de la GPU en MHz |
| `14 - 15` | GPU Fan Speed | `u16` (Big-Endian) | Velocidad del ventilador GPU en RPM |
| `16 - 17` | GPU Power | `u16` (Big-Endian) | Consumo GPU en Watts |
| `18 - 23` | Reservado | `[u8; 6]` | Bytes `0x00` |
| `24 - 25` | Año (Clock) | `u16` (Big-Endian) | Año actual (ej. `2026`) |
| `26` | Mes | `u8` | Mes actual (1 - 12) |
| `27` | Día | `u8` | Día del mes (1 - 31) |
| `28` | Hora | `u8` | Hora (0 - 23) |
| `29` | Minuto | `u8` | Minuto (0 - 59) |
| `30` | **Checksum** | `u8` | `sum(packet[1..=29]) & 0xFF` |
| `31 - 64` | Padding | `[u8; 34]` | Relleno de ceros `0x00` |

---

## Instalación en Ubuntu / Linux

Para instalarlo directamente desde el repositorio en una sola línea de comandos:

```bash
git clone https://github.com/ZarateBM/thermax-pulsar-linux.git
cd thermax-pulsar-linux
sudo ./install.sh
```

El instalador se encargará de todo de forma automática:
1. Compilar el binario nativo optimizado en **Rust** (o utilizar el binario precompilado).
2. Instalar el ejecutable en `/usr/local/bin/thermax-pulsar`.
3. Instalar la aplicación en tu **Cajón de Aplicaciones** con su icono oficial (`/usr/share/applications/thermax-pulsar.desktop`).
4. Crear la configuración en `/etc/thermax-pulsar/config.toml` (si ya existe una, se conserva).
5. Instalar las reglas `udev` para que el dispositivo USB funcione con permisos seguros.
6. Crear, habilitar y arrancar el servicio `systemd` para inicio automático con el sistema.

### ¿Cómo actualizar una instalación existente?

Si ya lo tienes instalado y quieres actualizar a la última versión:

```bash
cd thermax-pulsar-linux
git pull
sudo ./install.sh
```

- **Tu configuración se preserva:** El instalador detecta `/etc/thermax-pulsar/config.toml` y **no lo sobreescribe**.
- **Reinicio automático:** El servicio de fondo `thermax-pulsar.service` se reinicia automáticamente con la nueva versión compilada.

---

## 🖥️ Aplicación de Escritorio

Al instalar THERMAX 6 PULSAR, se registra automáticamente como una **aplicación de escritorio nativa**:

- **Desde el menú del sistema:** Abre el cajón de aplicaciones de Ubuntu (Super / Inicio), busca **THERMAX 6 PULSAR** y haz clic en su icono oficial.
- **Desde la terminal:** También puedes abrir la ventana directamente con:
  ```bash
  thermax-pulsar app
  # o también:
  thermax-pulsar gui
  ```

Se abrirá una ventana de aplicación dedicada e independiente (sin barra de direcciones web ni pestañas de navegador) que te permite:
- **Monitor en Tiempo Real:** Dials animados de CPU y GPU, frecuencia, velocidad de ventilador, porcentaje de uso y consumo de energía en Watts.
- **Ajustes de Configuración:** Cambia el modo de visualización (`cpu`, `gpu`, `alternate`, `highest`), unidad (`°C` / `°F`), intervalos de actualización y offsets.
- **Gestión del Servicio:** Iniciar, reiniciar o detener el servicio en segundo plano y ver logs en tiempo real.

*(Opcional: Si deseas ejecutarlo en modo servidor para acceder desde otro equipo en la red local, puedes usar `thermax-pulsar web --bind 0.0.0.0:9876`).*


---

## Comandos de Gestión del Servicio

- **Ver estado en tiempo real:**
  ```bash
  sudo systemctl status thermax-pulsar
  ```

- **Ver registros (logs):**
  ```bash
  journalctl -u thermax-pulsar -f
  ```

- **Reiniciar servicio:**
  ```bash
  sudo systemctl restart thermax-pulsar
  ```

- **Detener servicio:**
  ```bash
  sudo systemctl stop thermax-pulsar
  ```

---

## Pruebas Manuales (Línea de Comandos)

Puedes ejecutar el programa manualmente para hacer pruebas rápidas:

```bash
# Mostrar un único reporte y salir (útil para verificar)
thermax-pulsar --once -v

# Forzar modo GPU en tiempo de ejecución
thermax-pulsar --mode gpu -v

# Forzar modo alternado cada 3 segundos
thermax-pulsar --mode alternate --interval 1000
```

---

## Desinstalación

Para desinstalar completamente el servicio y los archivos:
```bash
sudo ./uninstall.sh
```

---

## Licencia

MIT License.
