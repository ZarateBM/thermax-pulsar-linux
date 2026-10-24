use crate::cooler::Metrics;
use std::fs;
use std::path::Path;
use sysinfo::{Components, CpuRefreshKind, RefreshKind, System};

pub struct CpuMonitor {
    sys: System,
    components: Components,
    hwmon_temp_path: Option<String>,
    hwmon_fan_path: Option<String>,
}

impl CpuMonitor {
    pub fn new() -> Self {
        let refresh_kind = RefreshKind::nothing().with_cpu(CpuRefreshKind::everything());
        let sys = System::new_with_specifics(refresh_kind);
        let components = Components::new_with_refreshed_list();

        let mut monitor = Self {
            sys,
            components,
            hwmon_temp_path: None,
            hwmon_fan_path: None,
        };

        monitor.discover_hwmon_paths();
        monitor
    }

    fn discover_hwmon_paths(&mut self) {
        #[cfg(target_os = "linux")]
        {
            if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name_file = path.join("name");
                    let name = fs::read_to_string(name_file).unwrap_or_default().trim().to_lowercase();

                    // Typical CPU temperature drivers on Linux:
                    // k10temp (AMD), zenpower (AMD), coretemp (Intel)
                    if name == "coretemp" || name == "k10temp" || name == "zenpower" {
                        // Look for temp1_input or temp2_input
                        for i in 1..=10 {
                            let temp_input = path.join(format!("temp{}_input", i));
                            if temp_input.exists() {
                                self.hwmon_temp_path = Some(temp_input.to_string_lossy().to_string());
                                break;
                            }
                        }
                    }

                    // Look for fan input (e.g. nct6775, it87, asus_wmi, etc.)
                    for i in 1..=5 {
                        let fan_input = path.join(format!("fan{}_input", i));
                        if fan_input.exists() && self.hwmon_fan_path.is_none() {
                            self.hwmon_fan_path = Some(fan_input.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    pub fn read_metrics(&mut self, temp_offset: f32) -> Metrics {
        self.sys.refresh_cpu_all();
        self.components.refresh(true);

        // CPU Utilization
        let util = self.sys.global_cpu_usage();

        // CPU Frequency (average across cores in MHz)
        let cpus = self.sys.cpus();
        let freq_mhz = if !cpus.is_empty() {
            let total_freq: u64 = cpus.iter().map(|c| c.frequency()).sum();
            (total_freq / cpus.len() as u64) as u16
        } else {
            0
        };

        // CPU Temperature
        let mut temp = 0.0f32;

        #[cfg(target_os = "linux")]
        {
            if let Some(ref path) = self.hwmon_temp_path {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(milli_c) = content.trim().parse::<f32>() {
                        temp = milli_c / 1000.0;
                    }
                }
            }
        }

        // Fallback to sysinfo components
        if temp <= 0.0 {
            for comp in self.components.iter() {
                let label = comp.label().to_lowercase();
                if label.contains("cpu")
                    || label.contains("core")
                    || label.contains("tctl")
                    || label.contains("tdie")
                    || label.contains("package")
                {
                    if let Some(t) = comp.temperature() {
                        if t > temp {
                            temp = t;
                        }
                    }
                }
            }
        }

        // Fan RPM from hwmon
        let mut fan_rpm = 0u16;
        #[cfg(target_os = "linux")]
        {
            if let Some(ref path) = self.hwmon_fan_path {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(rpm) = content.trim().parse::<u16>() {
                        fan_rpm = rpm;
                    }
                }
            }
        }

        Metrics {
            temp: (temp + temp_offset).max(0.0),
            util_percent: util,
            freq_mhz,
            fan_rpm,
            power_watts: 0,
        }
    }
}
