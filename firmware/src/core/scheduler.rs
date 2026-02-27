/// Task scheduler for periodic operations.
///
/// Manages timed tasks for sensor reading, data transmission,
/// status reporting, and maintenance operations.

use crate::config;

/// Task identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskId {
    /// Read BME280 (temperature, humidity, pressure).
    ReadBme280,
    /// Read wind speed and direction.
    ReadWind,
    /// Read rain gauge.
    ReadRain,
    /// Read UV and light sensors.
    ReadUvLight,
    /// Run data pipeline and produce WeatherReading.
    ProcessData,
    /// Publish telemetry (MQTT + LoRa).
    PublishTelemetry,
    /// Publish device status.
    PublishStatus,
    /// Update BLE notifications.
    UpdateBle,
    /// Check for OTA updates.
    CheckOta,
    /// Feed watchdog timer.
    FeedWatchdog,
    /// Read battery level.
    ReadBattery,
    /// Read agriculture sensors (soil moisture, soil temp, leaf wetness).
    #[cfg(feature = "agriculture")]
    ReadAgriculture,
    /// Process agriculture analytics (ET, GDD, frost, irrigation).
    #[cfg(feature = "agriculture")]
    ProcessAgriculture,
    /// Read solar sensors (pyranometer, panel temperature).
    #[cfg(feature = "solar")]
    ReadSolar,
    /// Process solar analytics (yield, performance ratio).
    #[cfg(feature = "solar")]
    ProcessSolar,
}

/// A scheduled task.
#[derive(Debug, Clone, Copy)]
pub struct ScheduledTask {
    pub id: TaskId,
    /// Interval between executions in milliseconds.
    pub interval_ms: u64,
    /// Time of next execution (absolute ms since boot).
    pub next_run_ms: u64,
    /// Whether this task is enabled.
    pub enabled: bool,
    /// Number of times this task has run.
    pub run_count: u64,
    /// Last execution duration in microseconds (for profiling).
    pub last_duration_us: u32,
}

/// Maximum number of scheduled tasks (base 11 + industry tasks).
const MAX_TASKS: usize = 15;

/// Task scheduler.
pub struct Scheduler {
    tasks: heapless::Vec<ScheduledTask, MAX_TASKS>,
    current_time_ms: u64,
}

impl Scheduler {
    pub fn new() -> Self {
        let mut tasks: heapless::Vec<ScheduledTask, MAX_TASKS> = heapless::Vec::new();
        let base_tasks = [
            ScheduledTask {
                id: TaskId::ReadBme280,
                interval_ms: config::SENSOR_READ_INTERVAL_MS,
                next_run_ms: 0,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
            ScheduledTask {
                id: TaskId::ReadWind,
                interval_ms: config::WIND_READ_INTERVAL_MS,
                next_run_ms: 0,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
            ScheduledTask {
                id: TaskId::ReadRain,
                interval_ms: config::SENSOR_READ_INTERVAL_MS,
                next_run_ms: 0,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
            ScheduledTask {
                id: TaskId::ReadUvLight,
                interval_ms: config::UV_LIGHT_READ_INTERVAL_MS,
                next_run_ms: 0,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
            ScheduledTask {
                id: TaskId::ProcessData,
                interval_ms: config::SENSOR_READ_INTERVAL_MS,
                next_run_ms: 500, // offset to run after sensor reads
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
            ScheduledTask {
                id: TaskId::PublishTelemetry,
                interval_ms: config::MQTT_PUBLISH_INTERVAL_MS,
                next_run_ms: 1000,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
            ScheduledTask {
                id: TaskId::PublishStatus,
                interval_ms: config::STATUS_REPORT_INTERVAL_MS,
                next_run_ms: 2000,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
            ScheduledTask {
                id: TaskId::UpdateBle,
                interval_ms: 5000,
                next_run_ms: 1500,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
            ScheduledTask {
                id: TaskId::CheckOta,
                interval_ms: config::OTA_CHECK_INTERVAL_MS,
                next_run_ms: 60_000, // first check 1 min after boot
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
            ScheduledTask {
                id: TaskId::FeedWatchdog,
                interval_ms: config::WATCHDOG_TIMEOUT_MS / 3,
                next_run_ms: 0,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
            ScheduledTask {
                id: TaskId::ReadBattery,
                interval_ms: 60_000,
                next_run_ms: 5000,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            },
        ];

        for task in base_tasks {
            let _ = tasks.push(task);
        }

        // Agriculture industry tasks
        #[cfg(feature = "agriculture")]
        {
            let _ = tasks.push(ScheduledTask {
                id: TaskId::ReadAgriculture,
                interval_ms: config::SOIL_READ_INTERVAL_MS,
                next_run_ms: 3000,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            });
            let _ = tasks.push(ScheduledTask {
                id: TaskId::ProcessAgriculture,
                interval_ms: config::SOIL_READ_INTERVAL_MS,
                next_run_ms: 3500,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            });
        }

        // Solar industry tasks
        #[cfg(feature = "solar")]
        {
            let _ = tasks.push(ScheduledTask {
                id: TaskId::ReadSolar,
                interval_ms: config::SOLAR_READ_INTERVAL_MS,
                next_run_ms: 2000,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            });
            let _ = tasks.push(ScheduledTask {
                id: TaskId::ProcessSolar,
                interval_ms: config::SOLAR_READ_INTERVAL_MS,
                next_run_ms: 2500,
                enabled: true,
                run_count: 0,
                last_duration_us: 0,
            });
        }

        Self {
            tasks,
            current_time_ms: 0,
        }
    }

    /// Update the current time. Call this from the main loop with the system tick.
    pub fn update_time(&mut self, now_ms: u64) {
        self.current_time_ms = now_ms;
    }

    /// Get the next task that is due to run. Returns None if no tasks are ready.
    pub fn next_due_task(&mut self) -> Option<TaskId> {
        let mut earliest_idx = None;
        let mut earliest_time = u64::MAX;

        for (i, task) in self.tasks.iter().enumerate() {
            if task.enabled && task.next_run_ms <= self.current_time_ms {
                if task.next_run_ms < earliest_time {
                    earliest_time = task.next_run_ms;
                    earliest_idx = Some(i);
                }
            }
        }

        if let Some(idx) = earliest_idx {
            let task = &mut self.tasks[idx];
            let id = task.id;
            task.next_run_ms = self.current_time_ms + task.interval_ms;
            task.run_count += 1;
            Some(id)
        } else {
            None
        }
    }

    /// Report task completion with execution duration.
    pub fn report_duration(&mut self, id: TaskId, duration_us: u32) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.last_duration_us = duration_us;
        }
    }

    /// Update the interval for a specific task.
    pub fn set_interval(&mut self, id: TaskId, interval_ms: u64) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.interval_ms = interval_ms;
            log::info!("Scheduler: {:?} interval set to {}ms", id, interval_ms);
        }
    }

    /// Enable or disable a task.
    pub fn set_enabled(&mut self, id: TaskId, enabled: bool) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
            task.enabled = enabled;
        }
    }

    /// Get time until next task is due (for sleep calculation).
    pub fn time_until_next_ms(&self) -> u64 {
        self.tasks
            .iter()
            .filter(|t| t.enabled)
            .map(|t| t.next_run_ms.saturating_sub(self.current_time_ms))
            .min()
            .unwrap_or(1000)
    }

    /// Get the current system time.
    pub fn now_ms(&self) -> u64 {
        self.current_time_ms
    }

    /// Get task statistics for diagnostics.
    pub fn task_stats(&self, id: TaskId) -> Option<(u64, u32)> {
        self.tasks
            .iter()
            .find(|t| t.id == id)
            .map(|t| (t.run_count, t.last_duration_us))
    }
}
