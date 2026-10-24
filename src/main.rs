mod config;
mod cooler;
mod hardware;

use clap::Parser;
use config::{Config, DisplayMode, TempUnit};
use cooler::{CoolerDevice, Metrics};
use hardware::{CpuMonitor, GpuMonitor};
use hidapi::HidApi;
use log::{error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(
    name = "thermax-pulsar",
    author = "Brandon",
    version = "0.1.0",
    about = "Ultra-lightweight Linux daemon for THERMAX 6 PULSAR cooler display"
)]
struct Args {
    #[arg(short, long, help = "Path to config file")]
    config: Option<String>,

    #[arg(
        short,
        long,
        value_parser = ["cpu", "gpu", "alternate", "highest"],
        help = "Override display mode: cpu, gpu, alternate, highest"
    )]
    mode: Option<String>,

    #[arg(
        short,
        long,
        value_parser = ["c", "f", "C", "F"],
        help = "Temperature unit: C or F"
    )]
    unit: Option<String>,

    #[arg(short, long, help = "Update interval in milliseconds")]
    interval: Option<u64>,

    #[arg(long, help = "Send one report and exit immediately (test mode)")]
    once: bool,

    #[arg(short, long, help = "Enable verbose debug logging")]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    let log_level = if args.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    env_logger::Builder::from_default_env()
        .filter_level(log_level)
        .init();

    info!("Starting THERMAX 6 PULSAR Linux Driver v{}", env!("CARGO_PKG_VERSION"));

    // Find config file: CLI argument -> /etc/thermax-pulsar/config.toml -> ./config.toml
    let config_path = args.config.unwrap_or_else(|| {
        if std::path::Path::new("/etc/thermax-pulsar/config.toml").exists() {
            "/etc/thermax-pulsar/config.toml".to_string()
        } else if std::path::Path::new("config.toml").exists() {
            "config.toml".to_string()
        } else {
            "".to_string()
        }
    });

    let mut cfg = if !config_path.is_empty() {
        info!("Loading configuration from: {}", config_path);
        Config::load_from_file(&config_path)
    } else {
        info!("Using default configuration");
        Config::default()
    };

    // Apply CLI overrides
    if let Some(ref m) = args.mode {
        cfg.display_mode = match m.to_lowercase().as_str() {
            "gpu" => DisplayMode::Gpu,
            "alternate" => DisplayMode::Alternate,
            "highest" => DisplayMode::Highest,
            _ => DisplayMode::Cpu,
        };
    }

    if let Some(ref u) = args.unit {
        cfg.temperature_unit = match u.to_uppercase().as_str() {
            "F" => TempUnit::F,
            _ => TempUnit::C,
        };
    }

    if let Some(i) = args.interval {
        cfg.update_interval_ms = i.max(200);
    }

    info!(
        "Configuration: Mode={:?}, Unit={:?}, Interval={}ms",
        cfg.display_mode, cfg.temperature_unit, cfg.update_interval_ms
    );

    // Setup graceful termination
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        info!("Shutdown signal received. Exiting cleanly...");
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Initialize HID API
    let mut hid_api = match HidApi::new() {
        Ok(api) => api,
        Err(e) => {
            error!("Failed to initialize HID API: {}", e);
            std::process::exit(1);
        }
    };

    let mut cooler = CoolerDevice::new();
    let mut cpu_mon = CpuMonitor::new();
    let mut gpu_mon = GpuMonitor::new(&cfg.gpu_driver);

    let mut last_alternate_switch = Instant::now();
    let mut alternate_show_gpu = false;

    let is_fahrenheit = cfg.temperature_unit == TempUnit::F;
    let interval = Duration::from_millis(cfg.update_interval_ms);

    while running.load(Ordering::SeqCst) {
        if !cooler.is_connected() {
            if !cooler.connect(&mut hid_api) {
                warn!("THERMAX 6 PULSAR cooler not detected. Retrying in 2 seconds...");
                sleep(Duration::from_secs(2));
                continue;
            }
        }

        // Read metrics
        let cpu_metrics = cpu_mon.read_metrics(cfg.cpu_temp_offset);
        let gpu_metrics = gpu_mon.read_metrics(cfg.gpu_temp_offset);

        // Determine which metrics should be sent to the primary display
        let primary_metrics: Metrics = match cfg.display_mode {
            DisplayMode::Cpu => cpu_metrics.clone(),
            DisplayMode::Gpu => gpu_metrics.clone(),
            DisplayMode::Highest => {
                if gpu_metrics.temp > cpu_metrics.temp {
                    gpu_metrics.clone()
                } else {
                    cpu_metrics.clone()
                }
            }
            DisplayMode::Alternate => {
                if last_alternate_switch.elapsed() >= Duration::from_secs(cfg.alternate_interval_secs) {
                    alternate_show_gpu = !alternate_show_gpu;
                    last_alternate_switch = Instant::now();
                }
                if alternate_show_gpu {
                    gpu_metrics.clone()
                } else {
                    cpu_metrics.clone()
                }
            }
        };

        // Send packet
        if let Err(e) = cooler.send_metrics(&primary_metrics, &cpu_metrics, &gpu_metrics, is_fahrenheit) {
            warn!("Failed to send metrics to cooler: {}", e);
        }

        if args.once {
            info!("Single report sent successfully (--once). Exiting.");
            break;
        }

        sleep(interval);
    }

    info!("Driver stopped.");
}
