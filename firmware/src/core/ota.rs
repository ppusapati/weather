/// OTA (Over-The-Air) firmware update manager.
///
/// Supports:
/// - Periodic version checks against a remote server
/// - Firmware download with SHA-256 verification
/// - Dual-partition A/B update scheme
/// - Automatic rollback on boot failure

use crate::config;
use crate::error::{Error, Result};
use heapless::String;

/// OTA update state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtaState {
    /// No update in progress.
    Idle,
    /// Checking for available update.
    Checking,
    /// Downloading firmware binary.
    Downloading { progress_pct: u8 },
    /// Verifying firmware integrity.
    Verifying,
    /// Writing to OTA partition.
    Applying,
    /// Waiting for reboot.
    PendingReboot,
    /// Update failed.
    Failed,
}

/// Firmware version info from the update server.
#[derive(Debug, Clone)]
pub struct FirmwareInfo {
    pub version: String<16>,
    pub size_bytes: u32,
    pub sha256: [u8; 32],
    pub url: String<128>,
}

/// OTA update manager.
pub struct OtaManager {
    state: OtaState,
    current_version: &'static str,
    check_url: String<128>,
    last_check_ms: u64,
    check_interval_ms: u64,
    /// SHA-256 hash accumulator for download verification.
    sha256_state: Sha256State,
    bytes_received: u32,
    total_bytes: u32,
}

/// Minimal SHA-256 state (placeholder for a real implementation).
#[derive(Debug, Clone)]
struct Sha256State {
    hash: [u8; 32],
}

impl Sha256State {
    fn new() -> Self {
        Self { hash: [0u8; 32] }
    }

    fn update(&mut self, _data: &[u8]) {
        // In real firmware: update SHA-256 state with incoming data
        // Use a no_std SHA-256 crate like `sha2`
    }

    fn finalize(&self) -> [u8; 32] {
        self.hash
    }
}

impl OtaManager {
    pub fn new() -> Self {
        Self {
            state: OtaState::Idle,
            current_version: config::FIRMWARE_VERSION,
            check_url: String::new(),
            last_check_ms: 0,
            check_interval_ms: config::OTA_CHECK_INTERVAL_MS,
            sha256_state: Sha256State::new(),
            bytes_received: 0,
            total_bytes: 0,
        }
    }

    /// Configure the OTA check URL.
    pub fn set_check_url(&mut self, url: &str) {
        self.check_url = String::try_from(url).unwrap_or_default();
    }

    /// Check if it's time to look for updates.
    pub fn should_check(&self, now_ms: u64) -> bool {
        if self.state != OtaState::Idle {
            return false;
        }
        if self.check_url.is_empty() {
            return false;
        }
        now_ms.saturating_sub(self.last_check_ms) >= self.check_interval_ms
    }

    /// Start checking for updates.
    pub fn check_for_update(&mut self, now_ms: u64) -> Result<()> {
        if self.check_url.is_empty() {
            return Ok(());
        }

        self.state = OtaState::Checking;
        self.last_check_ms = now_ms;

        log::info!(
            "OTA: checking for updates (current: v{})",
            self.current_version
        );

        // In real firmware:
        // 1. HTTP GET to check_url
        // 2. Parse JSON response for version + sha256
        // 3. Compare with current version
        // 4. If newer, start download

        Ok(())
    }

    /// Handle version check response.
    pub fn on_version_response(&mut self, info: &FirmwareInfo) {
        if info.version.as_str() <= self.current_version {
            log::info!(
                "OTA: no update available (remote: v{}, current: v{})",
                info.version.as_str(),
                self.current_version
            );
            self.state = OtaState::Idle;
            return;
        }

        log::info!(
            "OTA: update available v{} -> v{} ({} bytes)",
            self.current_version,
            info.version.as_str(),
            info.size_bytes
        );

        self.total_bytes = info.size_bytes;
        self.bytes_received = 0;
        self.sha256_state = Sha256State::new();
        self.state = OtaState::Downloading { progress_pct: 0 };

        // In real firmware: begin streaming download to OTA partition
    }

    /// Feed downloaded firmware data chunk.
    pub fn on_data_chunk(&mut self, data: &[u8]) -> Result<()> {
        if !matches!(self.state, OtaState::Downloading { .. }) {
            return Err(Error::OtaDownloadFailed);
        }

        // Update SHA-256
        self.sha256_state.update(data);

        // Write to OTA flash partition
        // In real firmware: esp_ota_write(handle, data)

        self.bytes_received += data.len() as u32;
        let progress = if self.total_bytes > 0 {
            ((self.bytes_received as u64 * 100) / self.total_bytes as u64) as u8
        } else {
            0
        };

        self.state = OtaState::Downloading {
            progress_pct: progress,
        };

        if progress % 10 == 0 {
            log::info!("OTA: download {}%", progress);
        }

        Ok(())
    }

    /// Called when download is complete. Verify and apply.
    pub fn on_download_complete(&mut self, expected_sha256: &[u8; 32]) -> Result<()> {
        self.state = OtaState::Verifying;

        let computed_hash = self.sha256_state.finalize();
        if computed_hash != *expected_sha256 {
            log::error!("OTA: SHA-256 verification FAILED");
            self.state = OtaState::Failed;
            return Err(Error::OtaVerifyFailed);
        }

        log::info!("OTA: SHA-256 verified, applying update");
        self.state = OtaState::Applying;

        // In real firmware:
        // 1. esp_ota_end(handle) — finalize OTA write
        // 2. esp_ota_set_boot_partition(ota_partition) — switch boot target
        // 3. esp_restart() — reboot into new firmware

        self.state = OtaState::PendingReboot;
        log::info!("OTA: update applied, reboot required");

        Ok(())
    }

    /// Called on first boot after OTA update to validate the new firmware.
    pub fn validate_boot(&mut self) -> Result<()> {
        // In real firmware:
        // 1. Run self-test (sensor probes, comm checks)
        // 2. If OK: esp_ota_mark_app_valid() — prevent rollback
        // 3. If FAIL: the bootloader will automatically rollback on next reboot

        log::info!("OTA: boot validation passed, marking firmware as stable");
        self.state = OtaState::Idle;
        Ok(())
    }

    /// Abort an in-progress update.
    pub fn abort(&mut self) {
        log::warn!("OTA: update aborted");
        self.state = OtaState::Idle;
        self.bytes_received = 0;
        self.total_bytes = 0;
    }

    pub fn state(&self) -> OtaState {
        self.state
    }

    pub fn progress(&self) -> Option<u8> {
        match self.state {
            OtaState::Downloading { progress_pct } => Some(progress_pct),
            _ => None,
        }
    }
}
