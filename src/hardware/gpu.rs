use crate::cooler::Metrics;
use std::fs;
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
enum GpuBackend {
    Nvidia,
    Amd,
    Intel,
    None,
}

pub struct GpuMonitor {
    backend: GpuBackend,
    amd_temp_path: Option<String>,
    amd_busy_path: Option<String>,
}

impl GpuMonitor {
    pub fn new(driver_hint: &str) -> Self {
        let mut monitor = Self {
            backend: GpuBackend::None,
            amd_temp_path: None,
            amd_busy_path: None,
        };

        match driver_hint.to_lowercase().as_str() {
            "nvidia" => monitor.backend = GpuBackend::Nvidia,
            "amd" => {
                monitor.backend = GpuBackend::Amd;
                monitor.discover_amd();
            }
            "intel" => monitor.backend = GpuBackend::Intel,
            "none" => monitor.backend = GpuBackend::None,
            _ => {
                // Auto detect
                if monitor.detect_nvidia() {
                    monitor.backend = GpuBackend::Nvidia;
                } else if monitor.discover_amd() {
                    monitor.backend = GpuBackend::Amd;
                }
            }
        }

        monitor
    }

    fn detect_nvidia(&self) -> bool {
        Command::new("nvidia-smi")
            .arg("--version")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    fn discover_amd(&mut self) -> bool {
        #[cfg(target_os = "linux")]
        {
            if let Ok(entries) = fs::read_dir("/sys/class/drm") {
                for entry in entries.flatten() {
                    let path = entry.path().join("device/hwmon");
                    if path.exists() {
                        if let Ok(hwmon_entries) = fs::read_dir(path) {
                            for hwmon in hwmon_entries.flatten() {
                                let temp_file = hwmon.path().join("temp1_input");
                                if temp_file.exists() {
                                    self.amd_temp_path = Some(temp_file.to_string_lossy().to_string());
                                    let busy_file = entry.path().join("device/gpu_busy_percent");
                                    if busy_file.exists() {
                                        self.amd_busy_path = Some(busy_file.to_string_lossy().to_string());
                                    }
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    pub fn read_metrics(&mut self, temp_offset: f32) -> Metrics {
        match self.backend {
            GpuBackend::Nvidia => self.read_nvidia(temp_offset),
            GpuBackend::Amd => self.read_amd(temp_offset),
            _ => Metrics::default(),
        }
    }

    fn read_nvidia(&self, temp_offset: f32) -> Metrics {
        // Run: nvidia-smi --query-gpu=temperature.gpu,utilization.gpu,clocks.gr,power.draw --format=csv,noheader,nounits
        let output = match Command::new("nvidia-smi")
            .args([
                "--query-gpu=temperature.gpu,utilization.gpu,clocks.gr,power.draw",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
            _ => return Metrics::default(),
        };

        // Output format: "45, 12, 1250, 42.5"
        let line = match output.lines().next() {
            Some(l) => l.trim(),
            None => return Metrics::default(),
        };

        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        let temp: f32 = parts.get(0).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let util: f32 = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let freq: u16 = parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0);
        let power: f32 = parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(0.0);

        Metrics {
            temp: (temp + temp_offset).max(0.0),
            util_percent: util,
            freq_mhz: freq,
            fan_rpm: 0,
            power_watts: power.round().max(0.0) as u16,
        }
    }

    fn read_amd(&self, temp_offset: f32) -> Metrics {
        let mut temp = 0.0f32;
        let mut util = 0.0f32;

        #[cfg(target_os = "linux")]
        {
            if let Some(ref path) = self.amd_temp_path {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(milli_c) = content.trim().parse::<f32>() {
                        temp = milli_c / 1000.0;
                    }
                }
            }

            if let Some(ref path) = self.amd_busy_path {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(pct) = content.trim().parse::<f32>() {
                        util = pct;
                    }
                }
            }
        }

        Metrics {
            temp: (temp + temp_offset).max(0.0),
            util_percent: util,
            freq_mhz: 0,
            fan_rpm: 0,
            power_watts: 0,
        }
    }
}
