#!/usr/bin/env python3
"""
THERMAX 6 PULSAR - Linux Driver & Daemon (Python)
Driver y demonio de control para el display digital THERMAX 6 PULSAR.
Lee métricas del sistema (CPU / GPU) y las envía vía USB HID (/dev/hidraw).
"""

import sys
import os
import time
import glob
import signal
import argparse
import datetime
import subprocess

try:
    import tomllib
except ImportError:
    tomllib = None

VENDOR_ID = "5131"
PRODUCT_ID = "2007"
REPORT_SIZE = 64

running = True

def signal_handler(signum, frame):
    global running
    running = False

signal.signal(signal.SIGTERM, signal_handler)
signal.signal(signal.SIGINT, signal_handler)

def load_config(config_path=None):
    """Carga configuración desde toml o usa valores por defecto"""
    defaults = {
        "display_mode": "cpu",
        "temperature_unit": "C",
        "update_interval_ms": 1000,
        "alternate_interval_secs": 5,
        "cpu_temp_offset": 0.0,
        "gpu_temp_offset": 0.0,
        "gpu_driver": "auto"
    }

    search_paths = []
    if config_path:
        search_paths.append(config_path)
    search_paths.extend(["/etc/thermax-pulsar/config.toml", "config.toml"])

    for path in search_paths:
        if os.path.isfile(path) and tomllib:
            try:
                with open(path, "rb") as f:
                    cfg = tomllib.load(f)
                    defaults.update(cfg)
                    return defaults, path
            except Exception as e:
                print(f"[!] Error leyendo config {path}: {e}")
                break

    return defaults, None

def find_hidraw_device():
    """Busca automáticamente el nodo /dev/hidraw correspondiente al disipador"""
    for hid_path in sorted(glob.glob("/sys/class/hidraw/hidraw*")):
        uevent_file = os.path.join(hid_path, "device", "uevent")
        if os.path.exists(uevent_file):
            try:
                with open(uevent_file, "r") as f:
                    content = f.read().lower()
                    if VENDOR_ID.lower() in content and PRODUCT_ID.lower() in content:
                        dev_name = os.path.basename(hid_path)
                        return os.path.join("/dev", dev_name)
            except Exception:
                pass
    return None

def get_cpu_temp(offset=0.0):
    """Lee la temperatura máxima de CPU desde /sys/class/hwmon"""
    max_temp = 0.0
    for path in glob.glob("/sys/class/hwmon/hwmon*/name"):
        try:
            with open(path, "r") as f:
                name = f.read().strip()
            if name in ["coretemp", "k10temp", "zenpower", "cpu_thermal"]:
                hwmon_dir = os.path.dirname(path)
                for tfile in sorted(glob.glob(os.path.join(hwmon_dir, "temp*_input"))):
                    with open(tfile, "r") as tf:
                        val = float(tf.read().strip()) / 1000.0
                        if val > max_temp:
                            max_temp = val
        except Exception:
            pass
    if max_temp == 0.0:
        max_temp = 38.0
    return max_temp + offset

def get_cpu_utilization(prev_stat=None):
    """Calcula el porcentaje de uso de CPU desde /proc/stat"""
    try:
        with open("/proc/stat", "r") as f:
            fields = [float(x) for x in f.readline().strip().split()[1:]]
        idle = fields[3] + fields[4]
        total = sum(fields)
        if prev_stat:
            prev_idle, prev_total = prev_stat
            diff_idle = idle - prev_idle
            diff_total = total - prev_total
            if diff_total > 0:
                util = 100.0 * (1.0 - diff_idle / diff_total)
                return max(0.0, min(100.0, util)), (idle, total)
        return 0.0, (idle, total)
    except Exception:
        return 0.0, None

def get_gpu_metrics(offset=0.0):
    """Lee métricas de GPU NVIDIA mediante nvidia-smi o AMD/Intel sysfs"""
    metrics = {"temp": 0.0, "util": 0.0, "freq": 0, "fan": 0, "power": 0}
    try:
        out = subprocess.check_output(
            ["nvidia-smi", "--query-gpu=temperature.gpu,utilization.gpu,clocks.gr,fan.speed,power.draw",
             "--format=csv,noheader,nounits"],
            stderr=subprocess.DEVNULL,
            text=True
        )
        parts = [p.strip() for p in out.strip().split(",")]
        if len(parts) >= 5:
            metrics["temp"] = float(parts[0]) + offset
            metrics["util"] = float(parts[1])
            metrics["freq"] = int(float(parts[2]))
            metrics["fan"] = int(float(parts[3]))
            metrics["power"] = int(float(parts[4]))
            return metrics
    except Exception:
        pass

    # AMD / Sysfs fallback
    for p in glob.glob("/sys/class/drm/card*/device/hwmon/hwmon*/temp1_input"):
        try:
            with open(p, "r") as f:
                metrics["temp"] = (float(f.read().strip()) / 1000.0) + offset
                break
        except Exception:
            pass

    return metrics

def build_packet(cpu_temp, cpu_util, gpu_temp, gpu_util, primary_temp, is_fahrenheit=False):
    """Construye el reporte HID de 64 bytes según el protocolo"""
    packet = bytearray(REPORT_SIZE)

    # Magic byte
    packet[0] = 0x40

    # Temperatura principal
    if is_fahrenheit:
        p_val = int(round((primary_temp * 1.8) + 32.0)) & 0x7F
        packet[1] = p_val | 0x80
    else:
        packet[1] = max(0, min(255, int(round(primary_temp))))

    # Utilización CPU
    packet[2] = max(0, min(100, int(round(cpu_util))))

    # Estimaciones CPU Freq, Fan, Power (pueden ser placeholders o refinados)
    packet[3] = 0x0D
    packet[4] = 0xAC
    packet[5] = 0x04
    packet[6] = 0xB0
    packet[7] = 0x00
    packet[8] = 0x41

    # Temperatura GPU
    if is_fahrenheit:
        g_val = int(round((gpu_temp * 1.8) + 32.0)) & 0x7F
        packet[9] = g_val | 0x80
    else:
        packet[9] = max(0, min(255, int(round(gpu_temp))))

    # Utilización GPU
    packet[10] = max(0, min(100, int(round(gpu_util))))

    # Fecha y Hora sincronizada con el reloj del display
    now = datetime.datetime.now()
    year = now.year
    packet[23] = (year >> 8) & 0xFF
    packet[24] = year & 0xFF
    packet[25] = now.month
    packet[26] = now.day
    packet[27] = now.hour
    packet[28] = now.minute

    # Checksum = sum(packet[0..28]) & 0xFF
    checksum = sum(packet[0:29]) & 0xFF
    packet[29] = checksum

    return packet

def main():
    parser = argparse.ArgumentParser(description="THERMAX 6 PULSAR Linux Driver")
    parser.add_argument("--config", "-c", help="Ruta al archivo de configuración (config.toml)")
    parser.add_argument("--device", "-d", help="Ruta manual al dispositivo (ej: /dev/hidraw2)")
    parser.add_argument("--mode", "-m", choices=["cpu", "gpu", "alternate", "highest"], help="Modo del display")
    parser.add_argument("--temp", "-t", type=float, help="Forzar temperatura fija para pruebas")
    parser.add_argument("--unit", "-u", choices=["C", "F", "c", "f"], help="Unidad de temperatura: C o F")
    parser.add_argument("--interval", "-i", type=float, help="Intervalo de actualización en segundos")
    parser.add_argument("--once", action="store_true", help="Enviar un solo paquete y salir")
    parser.add_argument("--verbose", "-v", action="store_true", help="Mostrar registros detallados")
    args = parser.parse_args()

    cfg, loaded_from = load_config(args.config)
    if args.verbose:
        if loaded_from:
            print(f"[*] Configuración cargada desde: {loaded_from}")
        else:
            print("[*] Usando configuración predeterminada")

    # CLI Overrides
    display_mode = (args.mode or cfg.get("display_mode", "cpu")).lower()
    temp_unit = (args.unit or cfg.get("temperature_unit", "C")).upper()
    interval_sec = (args.interval if args.interval is not None else float(cfg.get("update_interval_ms", 1000)) / 1000.0)
    alt_interval_sec = float(cfg.get("alternate_interval_secs", 5))
    cpu_offset = float(cfg.get("cpu_temp_offset", 0.0))
    gpu_offset = float(cfg.get("gpu_temp_offset", 0.0))
    is_fahrenheit = (temp_unit == "F")

    dev_path = args.device or find_hidraw_device()
    fd = None

    stat_prev = None
    alternate_state = False
    last_switch = time.time()

    print(f"❄️ THERMAX 6 PULSAR Driver iniciado [Modo: {display_mode.upper()}, Unidad: °{temp_unit}, Intervalo: {interval_sec}s]")

    while running:
        if fd is None:
            dev_path = args.device or find_hidraw_device()
            if not dev_path:
                if args.verbose:
                    print("[!] Esperando conexión de THERMAX 6 PULSAR...")
                time.sleep(2)
                continue
            try:
                fd = os.open(dev_path, os.O_RDWR)
                if args.verbose:
                    print(f"[*] Conectado exitosamente a {dev_path}")
            except Exception as e:
                print(f"[!] Error abriendo {dev_path}: {e}")
                time.sleep(2)
                continue

        # Lectura de métricas
        cpu_temp = get_cpu_temp(cpu_offset)
        cpu_util, stat_prev = get_cpu_utilization(stat_prev)
        gpu = get_gpu_metrics(gpu_offset)
        gpu_temp = gpu["temp"]
        gpu_util = gpu["util"]

        # Determinación de temperatura principal
        current_shown_mode = display_mode
        if args.temp is not None:
            primary_temp = args.temp
        elif display_mode == "gpu":
            primary_temp = gpu_temp
        elif display_mode == "highest":
            if gpu_temp > cpu_temp:
                primary_temp = gpu_temp
                current_shown_mode = "gpu"
            else:
                primary_temp = cpu_temp
                current_shown_mode = "cpu"
        elif display_mode == "alternate":
            if time.time() - last_switch >= alt_interval_sec:
                alternate_state = not alternate_state
                last_switch = time.time()
            if alternate_state:
                primary_temp = gpu_temp
                current_shown_mode = "gpu"
            else:
                primary_temp = cpu_temp
                current_shown_mode = "cpu"
        else:
            primary_temp = cpu_temp
            current_shown_mode = "cpu"

        packet = build_packet(
            cpu_temp=cpu_temp,
            cpu_util=cpu_util,
            gpu_temp=gpu_temp,
            gpu_util=gpu_util,
            primary_temp=primary_temp,
            is_fahrenheit=is_fahrenheit
        )

        try:
            os.write(fd, packet)
            if args.verbose:
                print(f"[{datetime.datetime.now().strftime('%H:%M:%S')}] Enviado -> {primary_temp:.1f}°{temp_unit} ({current_shown_mode.upper()}) | CPU: {cpu_temp:.1f}°C | GPU: {gpu_temp:.1f}°C")
        except Exception as e:
            print(f"[!] Error enviando reporte: {e}. Reconectando...")
            try:
                os.close(fd)
            except Exception:
                pass
            fd = None
            time.sleep(1)
            continue

        if args.once:
            break

        time.sleep(interval_sec)

    if fd is not None:
        try:
            os.close(fd)
        except Exception:
            pass

    print("\nDriver detenido limpiamente.")

if __name__ == "__main__":
    main()
