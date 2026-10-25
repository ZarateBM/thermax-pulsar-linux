use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    Cpu,
    Gpu,
    Alternate,
    Highest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TempUnit {
    C,
    F,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_display_mode")]
    pub display_mode: DisplayMode,

    #[serde(default = "default_unit")]
    pub temperature_unit: TempUnit,

    #[serde(default = "default_interval")]
    pub update_interval_ms: u64,

    #[serde(default = "default_alternate_interval")]
    pub alternate_interval_secs: u64,

    #[serde(default = "default_gpu_driver")]
    pub gpu_driver: String,

    #[serde(default)]
    pub cpu_temp_offset: f32,

    #[serde(default)]
    pub gpu_temp_offset: f32,
}

fn default_display_mode() -> DisplayMode {
    DisplayMode::Cpu
}

fn default_unit() -> TempUnit {
    TempUnit::C
}

fn default_interval() -> u64 {
    1000
}

fn default_alternate_interval() -> u64 {
    5
}

fn default_gpu_driver() -> String {
    "auto".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            display_mode: default_display_mode(),
            temperature_unit: default_unit(),
            update_interval_ms: default_interval(),
            alternate_interval_secs: default_alternate_interval(),
            gpu_driver: default_gpu_driver(),
            cpu_temp_offset: 0.0,
            gpu_temp_offset: 0.0,
        }
    }
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Self {
        if let Ok(content) = fs::read_to_string(path) {
            match toml::from_str(&content) {
                Ok(cfg) => return cfg,
                Err(e) => {
                    log::warn!("Failed to parse config file, using defaults: {}", e);
                }
            }
        }
        Config::default()
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        Ok(())
    }
}
