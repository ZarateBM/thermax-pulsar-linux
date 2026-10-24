use chrono::Local;
use hidapi::{HidApi, HidDevice};
use log::{debug, error, info, warn};
use std::thread::sleep;
use std::time::Duration;

pub const VENDOR_ID: u16 = 0x5131;
pub const PRODUCT_ID: u16 = 0x2007;
pub const REPORT_SIZE: usize = 65;

#[derive(Debug, Default, Clone)]
pub struct Metrics {
    pub temp: f32,
    pub util_percent: f32,
    pub freq_mhz: u16,
    pub fan_rpm: u16,
    pub power_watts: u16,
}

pub struct CoolerDevice {
    device: Option<HidDevice>,
}

impl CoolerDevice {
    pub fn new() -> Self {
        Self { device: None }
    }

    pub fn connect(&mut self, api: &mut HidApi) -> bool {
        if self.device.is_some() {
            return true;
        }

        // Refresh enumerated devices list
        if let Err(e) = api.refresh_devices() {
            warn!("Failed to refresh HID devices: {}", e);
        }

        for device_info in api.device_list() {
            if device_info.vendor_id() == VENDOR_ID && device_info.product_id() == PRODUCT_ID {
                info!("Found THERMAX 6 PULSAR at {:?}", device_info.path());
                match device_info.open_device(api) {
                    Ok(dev) => {
                        info!("Successfully opened THERMAX 6 PULSAR device!");
                        self.device = Some(dev);
                        return true;
                    }
                    Err(e) => {
                        error!("Failed to open THERMAX 6 PULSAR device: {}. Check udev rules/permissions.", e);
                    }
                }
            }
        }

        false
    }

    pub fn is_connected(&self) -> bool {
        self.device.is_some()
    }

    pub fn disconnect(&mut self) {
        self.device = None;
    }

    pub fn send_metrics(
        &mut self,
        primary_metrics: &Metrics,
        cpu_metrics: &Metrics,
        gpu_metrics: &Metrics,
        is_fahrenheit: bool,
    ) -> Result<(), String> {
        let dev = match self.device.as_mut() {
            Some(d) => d,
            None => return Err("Device not connected".to_string()),
        };

        let mut packet = [0u8; REPORT_SIZE];

        // Report ID
        packet[0] = 0x00;
        // Magic Byte
        packet[1] = 0x40;

        // Byte 2: Primary display temperature (can be CPU or GPU based on user selection)
        // Note: bit 7 (0x80) indicates Fahrenheit
        let mut primary_temp = primary_metrics.temp.round().clamp(0.0, 255.0) as u8;
        if is_fahrenheit {
            let temp_f = ((primary_metrics.temp * 1.8) + 32.0).round().clamp(0.0, 127.0) as u8;
            primary_temp = temp_f | 0x80;
        }
        packet[2] = primary_temp;

        // Byte 3: Utilization (percent 0-100)
        packet[3] = primary_metrics.util_percent.round().clamp(0.0, 100.0) as u8;

        // Bytes 4-5: Frequency (MHz big endian)
        let freq = primary_metrics.freq_mhz;
        packet[4] = ((freq >> 8) & 0xFF) as u8;
        packet[5] = (freq & 0xFF) as u8;

        // Bytes 6-7: Fan RPM (big endian)
        let fan = primary_metrics.fan_rpm;
        packet[6] = ((fan >> 8) & 0xFF) as u8;
        packet[7] = (fan & 0xFF) as u8;

        // Bytes 8-9: Power (Watts big endian)
        let power = primary_metrics.power_watts;
        packet[8] = ((power >> 8) & 0xFF) as u8;
        packet[9] = (power & 0xFF) as u8;

        // Byte 10: Secondary (GPU) Temperature
        let mut gpu_temp = gpu_metrics.temp.round().clamp(0.0, 255.0) as u8;
        if is_fahrenheit {
            let gpu_temp_f = ((gpu_metrics.temp * 1.8) + 32.0).round().clamp(0.0, 127.0) as u8;
            gpu_temp = gpu_temp_f | 0x80;
        }
        packet[10] = gpu_temp;

        // Byte 11: GPU Utilization
        packet[11] = gpu_metrics.util_percent.round().clamp(0.0, 100.0) as u8;

        // Bytes 12-13: GPU Frequency (MHz)
        let gpu_freq = gpu_metrics.freq_mhz;
        packet[12] = ((gpu_freq >> 8) & 0xFF) as u8;
        packet[13] = (gpu_freq & 0xFF) as u8;

        // Bytes 14-15: GPU Fan
        let gpu_fan = gpu_metrics.fan_rpm;
        packet[14] = ((gpu_fan >> 8) & 0xFF) as u8;
        packet[15] = (gpu_fan & 0xFF) as u8;

        // Bytes 16-17: GPU Power
        let gpu_power = gpu_metrics.power_watts;
        packet[16] = ((gpu_power >> 8) & 0xFF) as u8;
        packet[17] = (gpu_power & 0xFF) as u8;

        // Bytes 18-23: Reserved (all 0x00)

        // Date and Time (display clock calendar)
        let now = Local::now();
        let year = now.format("%Y").to_string().parse::<u16>().unwrap_or(2026);
        let month = now.format("%m").to_string().parse::<u8>().unwrap_or(1);
        let day = now.format("%d").to_string().parse::<u8>().unwrap_or(1);
        let hour = now.format("%H").to_string().parse::<u8>().unwrap_or(0);
        let minute = now.format("%M").to_string().parse::<u8>().unwrap_or(0);

        packet[24] = ((year >> 8) & 0xFF) as u8;
        packet[25] = (year & 0xFF) as u8;
        packet[26] = month;
        packet[27] = day;
        packet[28] = hour;
        packet[29] = minute;

        // Byte 30: Checksum = sum(packet[1..=29]) & 0xFF
        let mut checksum: u32 = 0;
        for i in 1..=29 {
            checksum = checksum.wrapping_add(packet[i] as u32);
        }
        packet[30] = (checksum & 0xFF) as u8;

        debug!(
            "Sending packet: CPU: {:.1}°C, GPU: {:.1}°C, Clock: {:04}-{:02}-{:02} {:02}:{:02}, Checksum: 0x{:02X}",
            cpu_metrics.temp, gpu_metrics.temp, year, month, day, hour, minute, packet[30]
        );

        match dev.write(&packet) {
            Ok(bytes) => {
                if bytes == REPORT_SIZE {
                    Ok(())
                } else {
                    Err(format!("Incomplete write: {}/{} bytes", bytes, REPORT_SIZE))
                }
            }
            Err(e) => {
                error!("HID write error: {}", e);
                self.disconnect();
                Err(e.to_string())
            }
        }
    }
}
