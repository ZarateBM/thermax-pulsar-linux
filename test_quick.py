#!/usr/bin/env python3
"""
THERMAX 6 PULSAR - Quick Test Script (Python)
Prueba rápida y directa del display digital sin necesidad de compilar Rust ni instalar servicios.

Uso:
    sudo python3 test_quick.py
    sudo python3 test_quick.py --once
    sudo python3 test_quick.py --temp 45
"""

import sys
import os
import time
import glob
import argparse
import datetime
import subprocess

VENDOR_ID = "5131"
PRODUCT_ID = "2007"
REPORT_SIZE = 64

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

def get_cpu_temp():
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
    return max_temp if max_temp > 0 else 40.0

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

def get_gpu_metrics():
    """Lee métricas de GPU NVIDIA mediante nvidia-smi"""
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
            metrics["temp"] = float(parts[0])
            metrics["util"] = float(parts[1])
            metrics["freq"] = int(float(parts[2]))
            metrics["fan"] = int(float(parts[3]))
            metrics["power"] = int(float(parts[4]))
    except Exception:
        pass
    return metrics

def build_packet(cpu_temp, cpu_util, gpu_temp, gpu_util, display_mode="cpu", force_temp=None, unit="C"):
    """Construye el reporte HID de 64 bytes según la ingeniería inversa"""
    packet = bytearray(REPORT_SIZE)

    # 1. Magic byte
    packet[0] = 0x40

    # 2. Temperatura principal del dial
    if force_temp is not None:
        primary_temp = float(force_temp)
    elif display_mode == "gpu":
        primary_temp = gpu_temp
    else:
        primary_temp = cpu_temp

    is_fahrenheit = (unit.upper() == "F")
    if is_fahrenheit:
        p_val = int(round((primary_temp * 1.8) + 32.0)) & 0x7F
        packet[1] = p_val | 0x80
    else:
        packet[1] = max(0, min(255, int(round(primary_temp))))

    # 3. Utilización CPU
    packet[2] = max(0, min(100, int(round(cpu_util))))

    # 4-5. Frecuencia CPU (MHz) - default aproximado si no se lee
    packet[3] = 0x0D  # ~3500 MHz
    packet[4] = 0xAC

    # 6-7. Fan RPM CPU
    packet[5] = 0x04  # ~1200 RPM
    packet[6] = 0xB0

    # 8-9. CPU Power (Watts)
    packet[7] = 0x00
    packet[8] = 0x41  # 65W

    # 10. Temperatura GPU
    if is_fahrenheit:
        g_val = int(round((gpu_temp * 1.8) + 32.0)) & 0x7F
        packet[9] = g_val | 0x80
    else:
        packet[9] = max(0, min(255, int(round(gpu_temp))))

    # 11. Utilización GPU
    packet[10] = max(0, min(100, int(round(gpu_util))))

    # 12-17. Reservados / Métricas GPU adicionales (ceros o valores base)

    # 23-28. Fecha y Hora actual (reloj del display)
    now = datetime.datetime.now()
    year = now.year
    packet[23] = (year >> 8) & 0xFF
    packet[24] = year & 0xFF
    packet[25] = now.month
    packet[26] = now.day
    packet[27] = now.hour
    packet[28] = now.minute

    # 29. Checksum = sum(packet[0..28]) & 0xFF
    checksum = sum(packet[0:29]) & 0xFF
    packet[29] = checksum

    return packet

def main():
    parser = argparse.ArgumentParser(description="Prueba rápida sin instalación del display THERMAX 6 PULSAR")
    parser.add_argument("--device", "-d", help="Ruta manual al dispositivo (ej: /dev/hidraw2)")
    parser.add_argument("--mode", "-m", choices=["cpu", "gpu", "alternate"], default="cpu", help="Modo del display (default: cpu)")
    parser.add_argument("--temp", "-t", type=float, help="Forzar una temperatura de prueba (ej: 50.0)")
    parser.add_argument("--unit", "-u", choices=["C", "F"], default="C", help="Unidad de temperatura (default: C)")
    parser.add_argument("--interval", "-i", type=float, default=1.0, help="Intervalo de actualización en segundos (default: 1.0)")
    parser.add_argument("--once", action="store_true", help="Enviar un solo paquete de prueba y salir")
    args = parser.parse_args()

    print("=" * 60)
    print(" ❄️  THERMAX 6 PULSAR - Prueba Rápida de Display")
    print("=" * 60)

    # 1. Encontrar dispositivo
    dev_path = args.device or find_hidraw_device()
    if not dev_path:
        print("\n❌ ERROR: No se encontró el dispositivo THERMAX 6 PULSAR (VID: 0x5131, PID: 0x2007).")
        print("   Verifica que el cable USB interno del disipador esté conectado a la placa madre.")
        print("   Revisa con: lsusb | grep 5131")
        sys.exit(1)

    print(f"✅ Dispositivo detectado: {dev_path}")

    # 2. Abrir dispositivo
    try:
        fd = os.open(dev_path, os.O_RDWR)
    except PermissionError:
        print(f"\n❌ ERROR DE PERMISOS: No se puede escribir en {dev_path}.")
        print("   Para probar sin instalar reglas udev, ejecuta con sudo:")
        print(f"   👉  sudo python3 test_quick.py")
        print("   O dale permisos temporales:")
        print(f"   👉  sudo chmod 666 {dev_path}")
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Error al abrir {dev_path}: {e}")
        sys.exit(1)

    print(f"✅ Comunicación abierta con el disipador.")
    print(f"ℹ️  Modo: {args.mode.upper()} | Unidad: °{args.unit} | Intervalo: {args.interval}s")
    if args.temp:
        print(f"⚠️  Temperatura fija de prueba: {args.temp}°{args.unit}")
    print("\nPresiona Ctrl+C para detener la prueba en cualquier momento.\n")

    stat_prev = None
    alternate_state = False
    last_switch = time.time()

    try:
        while True:
            # Lectura de métricas
            cpu_temp = get_cpu_temp()
            cpu_util, stat_prev = get_cpu_utilization(stat_prev)
            gpu = get_gpu_metrics()
            gpu_temp = gpu["temp"]
            gpu_util = gpu["util"]

            current_mode = args.mode
            if args.mode == "alternate":
                if time.time() - last_switch > 4.0:
                    alternate_state = not alternate_state
                    last_switch = time.time()
                current_mode = "gpu" if alternate_state else "cpu"

            # Construir paquete
            packet = build_packet(
                cpu_temp=cpu_temp,
                cpu_util=cpu_util,
                gpu_temp=gpu_temp,
                gpu_util=gpu_util,
                display_mode=current_mode,
                force_temp=args.temp,
                unit=args.unit
            )

            # Enviar paquete a /dev/hidraw
            written = os.write(fd, packet)

            shown_temp = args.temp if args.temp is not None else (gpu_temp if current_mode == "gpu" else cpu_temp)
            now_str = datetime.datetime.now().strftime("%H:%M:%S")

            print(f"[{now_str}] 📡 Enviado ({written} bytes) -> Display: {shown_temp:.1f}°{args.unit} ({current_mode.upper()}) | CPU: {cpu_temp:.1f}°C ({cpu_util:.0f}%) | GPU: {gpu_temp:.1f}°C ({gpu_util:.0f}%)")

            if args.once:
                print("\n✅ Paquete único enviado exitosamente (--once).")
                break

            time.sleep(args.interval)

    except KeyboardInterrupt:
        print("\n\n🛑 Prueba finalizada por el usuario.")
    finally:
        os.close(fd)

if __name__ == "__main__":
    main()
