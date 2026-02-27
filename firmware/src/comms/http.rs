/// HTTP REST API server.
///
/// Serves:
/// - GET  /              — Minimal HTML status page
/// - GET  /api/v1/current — Latest sensor readings (JSON)
/// - GET  /api/v1/history — Historical readings (JSON)
/// - GET  /api/v1/status  — Device status (JSON)
/// - POST /api/v1/config  — Update configuration (JSON)
/// - POST /api/v1/ota     — Upload firmware binary

use crate::config;
use crate::core::data_pipeline::WeatherReading;
use crate::drivers::SensorStatusMap;
use crate::error::{Error, Result};
use serde::Serialize;

/// HTTP server state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpState {
    Stopped,
    Running,
}

/// HTTP request method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Options,
    Unknown,
}

/// Parsed HTTP request.
pub struct HttpRequest<'a> {
    pub method: Method,
    pub path: &'a str,
    pub body: &'a [u8],
    pub content_length: usize,
}

/// HTTP response builder.
pub struct HttpResponse {
    pub status_code: u16,
    pub content_type: &'static str,
    pub body: ResponseBody,
}

pub enum ResponseBody {
    Json(heapless::Vec<u8, 1024>),
    Html(&'static str),
    Empty,
}

/// Current readings response.
#[derive(Debug, Serialize)]
pub struct CurrentReadingsResponse {
    pub timestamp: u64,
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub pressure_hpa: Option<f32>,
    pub wind_speed_kmh: Option<f32>,
    pub wind_direction_deg: Option<u16>,
    pub rain_mm_hour: Option<f32>,
    pub rain_mm_total: f32,
    pub uv_index: Option<f32>,
    pub light_lux: Option<f32>,
}

/// Device status response.
#[derive(Debug, Serialize)]
pub struct StatusResponse<'a> {
    pub uptime_s: u64,
    pub firmware_version: &'a str,
    pub battery_pct: u8,
    pub wifi_rssi_dbm: i8,
    pub free_heap_bytes: u32,
    pub sensor_status: &'a SensorStatusMap,
}

/// HTTP REST API server.
pub struct HttpServer {
    state: HttpState,
    port: u16,
    cached_reading: Option<WeatherReading>,
    cached_status: Option<SensorStatusMap>,
}

impl HttpServer {
    pub fn new() -> Self {
        Self {
            state: HttpState::Stopped,
            port: config::HTTP_PORT,
            cached_reading: None,
            cached_status: None,
        }
    }

    /// Start the HTTP server. Requires WiFi to be connected.
    pub fn start(&mut self) -> Result<()> {
        log::info!("HTTP: starting server on port {}", self.port);

        // In real firmware:
        // 1. Bind TCP listener to port 80
        // 2. Accept connections (up to MAX_CONNECTIONS)
        // 3. Spawn handler task per connection

        self.state = HttpState::Running;
        Ok(())
    }

    /// Stop the HTTP server.
    pub fn stop(&mut self) {
        self.state = HttpState::Stopped;
        log::info!("HTTP: server stopped");
    }

    /// Update the cached reading for API responses.
    pub fn update_reading(&mut self, reading: &WeatherReading) {
        self.cached_reading = Some(reading.clone());
    }

    /// Update the cached sensor status.
    pub fn update_status(&mut self, status: &SensorStatusMap) {
        self.cached_status = Some(status.clone());
    }

    /// Handle an incoming HTTP request and produce a response.
    pub fn handle_request(&self, request: &HttpRequest) -> HttpResponse {
        match (request.method, request.path) {
            (Method::Get, "/") => self.handle_index(),
            (Method::Get, "/api/v1/current") => self.handle_get_current(),
            (Method::Get, "/api/v1/status") => self.handle_get_status(),
            (Method::Post, "/api/v1/config") => self.handle_post_config(request.body),
            (Method::Post, "/api/v1/ota") => self.handle_post_ota(request.body),
            (Method::Options, _) => self.handle_cors_preflight(),
            _ => HttpResponse {
                status_code: 404,
                content_type: "application/json",
                body: ResponseBody::Json(
                    heapless::Vec::from_slice(b"{\"error\":\"not found\"}").unwrap_or_default(),
                ),
            },
        }
    }

    fn handle_index(&self) -> HttpResponse {
        let html = r#"<!DOCTYPE html>
<html><head><title>Weather Station</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{font-family:monospace;max-width:600px;margin:0 auto;padding:1em}
table{width:100%;border-collapse:collapse}td,th{padding:8px;border:1px solid #ddd;text-align:left}
h1{color:#2c3e50}</style></head>
<body><h1>Weather Station</h1>
<p>Firmware: v1.0.0</p>
<h2>API Endpoints</h2>
<table><tr><th>Method</th><th>Path</th><th>Description</th></tr>
<tr><td>GET</td><td>/api/v1/current</td><td>Current readings</td></tr>
<tr><td>GET</td><td>/api/v1/status</td><td>Device status</td></tr>
<tr><td>POST</td><td>/api/v1/config</td><td>Update config</td></tr>
<tr><td>POST</td><td>/api/v1/ota</td><td>Firmware update</td></tr>
</table></body></html>"#;

        HttpResponse {
            status_code: 200,
            content_type: "text/html",
            body: ResponseBody::Html(html),
        }
    }

    fn handle_get_current(&self) -> HttpResponse {
        match &self.cached_reading {
            Some(reading) => {
                let resp = CurrentReadingsResponse {
                    timestamp: reading.timestamp_ms,
                    temperature_c: reading.temperature_c,
                    humidity_pct: reading.humidity_pct,
                    pressure_hpa: reading.pressure_hpa,
                    wind_speed_kmh: reading.wind_speed_kmh,
                    wind_direction_deg: reading.wind_dir_deg,
                    rain_mm_hour: reading.rain_rate_mm_hr,
                    rain_mm_total: reading.rain_mm,
                    uv_index: reading.uv_index,
                    light_lux: reading.light_lux,
                };
                // In real firmware: serialize resp to JSON
                let _ = resp;
                HttpResponse {
                    status_code: 200,
                    content_type: "application/json",
                    body: ResponseBody::Json(
                        heapless::Vec::from_slice(b"{\"status\":\"ok\"}").unwrap_or_default(),
                    ),
                }
            }
            None => HttpResponse {
                status_code: 503,
                content_type: "application/json",
                body: ResponseBody::Json(
                    heapless::Vec::from_slice(b"{\"error\":\"no data yet\"}").unwrap_or_default(),
                ),
            },
        }
    }

    fn handle_get_status(&self) -> HttpResponse {
        HttpResponse {
            status_code: 200,
            content_type: "application/json",
            body: ResponseBody::Json(
                heapless::Vec::from_slice(
                    b"{\"firmware_version\":\"1.0.0\",\"status\":\"running\"}",
                )
                .unwrap_or_default(),
            ),
        }
    }

    fn handle_post_config(&self, body: &[u8]) -> HttpResponse {
        if body.is_empty() {
            return HttpResponse {
                status_code: 400,
                content_type: "application/json",
                body: ResponseBody::Json(
                    heapless::Vec::from_slice(b"{\"error\":\"empty body\"}").unwrap_or_default(),
                ),
            };
        }

        log::info!("HTTP: config update received ({} bytes)", body.len());

        // In real firmware: parse JSON, update RuntimeConfig, save to NVS
        HttpResponse {
            status_code: 200,
            content_type: "application/json",
            body: ResponseBody::Json(
                heapless::Vec::from_slice(b"{\"status\":\"ok\"}").unwrap_or_default(),
            ),
        }
    }

    fn handle_post_ota(&self, body: &[u8]) -> HttpResponse {
        if body.is_empty() {
            return HttpResponse {
                status_code: 400,
                content_type: "application/json",
                body: ResponseBody::Json(
                    heapless::Vec::from_slice(b"{\"error\":\"empty firmware\"}").unwrap_or_default(),
                ),
            };
        }

        log::info!("HTTP: OTA firmware received ({} bytes)", body.len());

        // In real firmware: verify SHA-256, write to OTA partition, reboot
        HttpResponse {
            status_code: 202,
            content_type: "application/json",
            body: ResponseBody::Json(
                heapless::Vec::from_slice(
                    b"{\"status\":\"accepted\",\"message\":\"device will reboot\"}",
                )
                .unwrap_or_default(),
            ),
        }
    }

    fn handle_cors_preflight(&self) -> HttpResponse {
        HttpResponse {
            status_code: 204,
            content_type: "text/plain",
            body: ResponseBody::Empty,
        }
    }

    /// Parse a raw HTTP request.
    pub fn parse_request<'a>(buf: &'a [u8]) -> Option<HttpRequest<'a>> {
        let text = core::str::from_utf8(buf).ok()?;
        let first_line = text.lines().next()?;
        let mut parts = first_line.split_whitespace();

        let method = match parts.next()? {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "OPTIONS" => Method::Options,
            _ => Method::Unknown,
        };

        let path = parts.next()?;

        // Find body (after \r\n\r\n)
        let body_start = text.find("\r\n\r\n").map(|i| i + 4).unwrap_or(text.len());
        let body = &buf[body_start.min(buf.len())..];

        Some(HttpRequest {
            method,
            path,
            body,
            content_length: body.len(),
        })
    }

    pub fn is_running(&self) -> bool {
        self.state == HttpState::Running
    }
}
