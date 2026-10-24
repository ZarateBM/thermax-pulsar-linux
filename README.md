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

<<<<<<< HEAD
## Instalación en Ubuntu / Linux (1 Solo Comando)
=======
## Instalación en Ubuntu 26 / Linux
>>>>>>> 89b8f3d5929763749225bae04cdc5b0791abf7d4

No necesitas instalar Rust ni configurar nada manualmente por separado. Ejecuta el instalador provisto:

```bash
sudo ./install.sh
```

El instalador te permitirá elegir:
- **1) Versión Python (Recomendada):** Instalación instantánea en 2 segundos, no requiere instalar Rust ni compilar nada (usa el Python nativo de Ubuntu).
- **2) Versión Rust:** Descarga e instala automáticamente Rust y las herramientas de compilación necesarias, y compila el binario nativo.

También puedes ejecutarlo de forma directa y no interactiva:
```bash
# Instalación instantánea con Python (0 descargas adicionales):
sudo ./install.sh --python

# O si prefieres la versión compilada en Rust:
sudo ./install.sh --rust
```

El script se encarga de:
1. Instalar el ejecutable en `/usr/local/bin/thermax-pulsar`.
2. Crear la configuración en `/etc/thermax-pulsar/config.toml`.
3. Instalar las reglas `udev` para que el dispositivo USB funcione con permisos seguros.
4. Crear, habilitar y arrancar el servicio `systemd` para que se inicie automáticamente con el sistema.

---

## Configuración

El archivo de configuración principal se encuentra en:
`/etc/thermax-pulsar/config.toml`

```toml
# Modo de visualización:
#  "cpu"       : Muestra la temperatura del CPU en el display
#  "gpu"       : Muestra la temperatura de la GPU en el display
#  "alternate" : Alterna entre CPU y GPU automáticamente
#  "highest"   : Muestra el componente con mayor temperatura
display_mode = "cpu"

# Unidad de temperatura: "C" o "F"
temperature_unit = "C"

# Frecuencia de actualización en milisegundos (1000 = 1 segundo)
update_interval_ms = 1000

# Segundos que permanece cada sensor al usar el modo "alternate"
alternate_interval_secs = 5

# Detección de GPU: "auto", "nvidia", "amd", "intel", "none"
gpu_driver = "auto"
```

Tras modificar la configuración, reinicia el servicio:
```bash
sudo systemctl restart thermax-pulsar
```

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
