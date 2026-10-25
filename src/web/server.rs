use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};

use crate::config::{Config, DisplayMode, TempUnit};
use crate::cooler::Metrics;
use crate::hardware::{CpuMonitor, GpuMonitor};

// ═══════════════════════════════════════════════════════════════
//  EMBEDDED STATIC FILES
// ═══════════════════════════════════════════════════════════════

const INDEX_HTML: &str = include_str!("static/index.html");
const STYLE_CSS: &str = include_str!("static/style.css");
const APP_JS: &str = include_str!("static/app.js");

// ═══════════════════════════════════════════════════════════════
//  SHARED STATE
// ═══════════════════════════════════════════════════════════════

pub struct AppState {
    config_path: String,
    config: Mutex<Config>,
    cpu_monitor: Mutex<CpuMonitor>,
    gpu_monitor: Mutex<GpuMonitor>,
}

// ═══════════════════════════════════════════════════════════════
//  API DATA TYPES
// ═══════════════════════════════════════════════════════════════

#[derive(Serialize)]
struct MetricsResponse {
    cpu: MetricsData,
    gpu: MetricsData,
}

#[derive(Serialize)]
struct MetricsData {
    temp: f32,
    util_percent: f32,
    freq_mhz: u16,
    fan_rpm: u16,
    power_watts: u16,
}

impl From<Metrics> for MetricsData {
    fn from(m: Metrics) -> Self {
        Self {
            temp: m.temp,
            util_percent: m.util_percent,
            freq_mhz: m.freq_mhz,
            fan_rpm: m.fan_rpm,
            power_watts: m.power_watts,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ConfigPayload {
    display_mode: String,
    temperature_unit: String,
    update_interval_ms: u64,
    alternate_interval_secs: u64,
    gpu_driver: String,
    cpu_temp_offset: f32,
    gpu_temp_offset: f32,
}

#[derive(Serialize)]
struct ServiceStatus {
    status: String,
    detail: String,
}

#[derive(Serialize)]
struct LogsResponse {
    logs: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// ═══════════════════════════════════════════════════════════════
//  PUBLIC: START WEB SERVER
// ═══════════════════════════════════════════════════════════════

pub async fn start_server(config_path: String, bind_addr: String) {
    // Load initial config
    let config = Config::load_from_file(&config_path);
    let gpu_driver = config.gpu_driver.clone();

    let state = Arc::new(AppState {
        config_path,
        config: Mutex::new(config),
        cpu_monitor: Mutex::new(CpuMonitor::new()),
        gpu_monitor: Mutex::new(GpuMonitor::new(&gpu_driver)),
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Static assets (embedded)
        .route("/", get(serve_index))
        .route("/static/style.css", get(serve_css))
        .route("/static/app.js", get(serve_js))
        // API endpoints
        .route("/api/metrics", get(api_get_metrics))
        .route("/api/config", get(api_get_config))
        .route("/api/config", post(api_set_config))
        .route("/api/service/status", get(api_service_status))
        .route("/api/service/start", post(api_service_start))
        .route("/api/service/stop", post(api_service_stop))
        .route("/api/service/restart", post(api_service_restart))
        .route("/api/service/logs", get(api_service_logs))
        .layer(cors)
        .with_state(state);

    info!("═══════════════════════════════════════════════════════════");
    info!("  THERMAX PULSAR — Web Control Panel");
    info!("  Listening on: http://{}", bind_addr);
    info!("═══════════════════════════════════════════════════════════");

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect(&format!("Failed to bind to {}", bind_addr));

    axum::serve(listener, app)
        .await
        .expect("Web server error");
}

// ═══════════════════════════════════════════════════════════════
//  STATIC FILE HANDLERS
// ═══════════════════════════════════════════════════════════════

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn serve_css() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLE_CSS,
    )
        .into_response()
}

async fn serve_js() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        APP_JS,
    )
        .into_response()
}

// ═══════════════════════════════════════════════════════════════
//  API: METRICS (Real-time hardware data)
// ═══════════════════════════════════════════════════════════════

async fn api_get_metrics(State(state): State<Arc<AppState>>) -> Json<MetricsResponse> {
    let cfg = state.config.lock().unwrap().clone();

    let cpu_metrics = {
        let mut cpu = state.cpu_monitor.lock().unwrap();
        cpu.read_metrics(cfg.cpu_temp_offset)
    };

    let gpu_metrics = {
        let mut gpu = state.gpu_monitor.lock().unwrap();
        gpu.read_metrics(cfg.gpu_temp_offset)
    };

    Json(MetricsResponse {
        cpu: cpu_metrics.into(),
        gpu: gpu_metrics.into(),
    })
}

// ═══════════════════════════════════════════════════════════════
//  API: CONFIG (Read / Write configuration)
// ═══════════════════════════════════════════════════════════════

async fn api_get_config(State(state): State<Arc<AppState>>) -> Json<ConfigPayload> {
    let cfg = state.config.lock().unwrap().clone();

    Json(ConfigPayload {
        display_mode: match cfg.display_mode {
            DisplayMode::Cpu => "cpu",
            DisplayMode::Gpu => "gpu",
            DisplayMode::Alternate => "alternate",
            DisplayMode::Highest => "highest",
        }
        .to_string(),
        temperature_unit: match cfg.temperature_unit {
            TempUnit::C => "C",
            TempUnit::F => "F",
        }
        .to_string(),
        update_interval_ms: cfg.update_interval_ms,
        alternate_interval_secs: cfg.alternate_interval_secs,
        gpu_driver: cfg.gpu_driver,
        cpu_temp_offset: cfg.cpu_temp_offset,
        gpu_temp_offset: cfg.gpu_temp_offset,
    })
}

async fn api_set_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ConfigPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Parse the payload into Config
    let new_config = Config {
        display_mode: match payload.display_mode.to_lowercase().as_str() {
            "gpu" => DisplayMode::Gpu,
            "alternate" => DisplayMode::Alternate,
            "highest" => DisplayMode::Highest,
            _ => DisplayMode::Cpu,
        },
        temperature_unit: match payload.temperature_unit.to_uppercase().as_str() {
            "F" => TempUnit::F,
            _ => TempUnit::C,
        },
        update_interval_ms: payload.update_interval_ms.max(250),
        alternate_interval_secs: payload.alternate_interval_secs.max(1),
        gpu_driver: payload.gpu_driver,
        cpu_temp_offset: payload.cpu_temp_offset,
        gpu_temp_offset: payload.gpu_temp_offset,
    };

    // Save to file
    if let Err(e) = new_config.save_to_file(&state.config_path) {
        error!("Failed to save config: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        ));
    }

    // Update in-memory config
    {
        let mut cfg = state.config.lock().unwrap();
        *cfg = new_config;
    }

    // Try to restart the systemd service so changes take effect
    let restart_result = try_restart_service();

    info!("Configuration saved to {}", state.config_path);

    Ok(Json(serde_json::json!({
        "ok": true,
        "service_restarted": restart_result
    })))
}

// ═══════════════════════════════════════════════════════════════
//  API: SERVICE CONTROL (systemd)
// ═══════════════════════════════════════════════════════════════

async fn api_service_status() -> Json<ServiceStatus> {
    let (status, detail) = get_systemd_status();
    Json(ServiceStatus { status, detail })
}

async fn api_service_start() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    run_systemctl("start")
}

async fn api_service_stop() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    run_systemctl("stop")
}

async fn api_service_restart() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    run_systemctl("restart")
}

async fn api_service_logs() -> Json<LogsResponse> {
    let logs = get_service_logs();
    Json(LogsResponse { logs })
}

// ═══════════════════════════════════════════════════════════════
//  SYSTEMD HELPERS
// ═══════════════════════════════════════════════════════════════

fn get_systemd_status() -> (String, String) {
    match Command::new("systemctl")
        .args(["is-active", "thermax-pulsar.service"])
        .output()
    {
        Ok(output) => {
            let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if status == "active" {
                "thermax-pulsar.service — Running".to_string()
            } else {
                format!("thermax-pulsar.service — {}", status)
            };
            (status, detail)
        }
        Err(_) => ("unknown".to_string(), "systemctl not available".to_string()),
    }
}

fn run_systemctl(
    action: &str,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match Command::new("systemctl")
        .args([action, "thermax-pulsar.service"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                info!("systemctl {} thermax-pulsar.service: success", action);
                Ok(Json(serde_json::json!({ "ok": true })))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                warn!("systemctl {} failed: {}", action, stderr);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("systemctl {} failed: {}", action, stderr),
                    }),
                ))
            }
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to execute systemctl: {}", e),
            }),
        )),
    }
}

fn try_restart_service() -> bool {
    match Command::new("systemctl")
        .args(["restart", "thermax-pulsar.service"])
        .output()
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

fn get_service_logs() -> String {
    match Command::new("journalctl")
        .args([
            "-u",
            "thermax-pulsar.service",
            "-n",
            "100",
            "--no-pager",
            "--output=short-iso",
        ])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                String::from_utf8_lossy(&output.stderr).to_string()
            }
        }
        Err(e) => format!("Error reading logs: {}", e),
    }
}
