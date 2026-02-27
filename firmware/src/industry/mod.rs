/// Industry-specific analytics modules.
///
/// Enable via Cargo features:
/// - `agriculture` — Soil monitoring, ET calculation, GDD tracking, frost alerts
/// - `solar` — Irradiance analytics, panel performance, yield estimation
/// - `all-industries` — Both agriculture and solar

#[cfg(feature = "agriculture")]
pub mod agriculture;
#[cfg(feature = "solar")]
pub mod solar;

use serde::{Deserialize, Serialize};

/// Alert severity levels used across all industry modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// An industry-specific alert ready for MQTT publication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryAlert {
    pub severity: AlertSeverity,
    pub category: heapless::String<32>,
    pub message: heapless::String<128>,
    pub timestamp_ms: u64,
}
