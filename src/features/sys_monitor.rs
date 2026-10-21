//! System Resource Monitor (Feature 5)
//!
//! Provides lightweight, on-demand CPU load and physical RAM metrics.
//! Designed strictly for 0.0% idle CPU overhead:
//! - Queries Win32 `GetSystemTimes` for CPU load delta
//! - Queries Win32 `GlobalMemoryStatusEx` for physical RAM usage
//! - Completely dormant when the island is collapsed; only polled on-demand when expanded.

use std::time::Instant;
use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

#[derive(Debug, Clone, PartialEq)]
pub struct SystemMetrics {
    pub cpu_usage_pct: f32,
    pub ram_usage_pct: u32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_usage_pct: 0.0,
            ram_usage_pct: 0,
            ram_used_gb: 0.0,
            ram_total_gb: 0.0,
        }
    }
}

pub struct SystemMonitor {
    last_idle: u64,
    last_kernel: u64,
    last_user: u64,
    last_sample_time: Option<Instant>,
    cached_metrics: SystemMetrics,
    cache_ttl_secs: f32,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut monitor = Self {
            last_idle: 0,
            last_kernel: 0,
            last_user: 0,
            last_sample_time: None,
            cached_metrics: SystemMetrics::default(),
            cache_ttl_secs: 1.5,
        };

        // Initialize baseline counters
        monitor.sample_raw();
        monitor
    }

    /// Returns current system metrics.
    /// Only performs Win32 API queries if the cache has expired (default: 1.5s).
    pub fn get_metrics(&mut self) -> &SystemMetrics {
        let should_refresh = match self.last_sample_time {
            Some(last) => last.elapsed().as_secs_f32() >= self.cache_ttl_secs,
            None => true,
        };

        if should_refresh {
            self.refresh_metrics();
        }

        &self.cached_metrics
    }

    fn filetime_to_u64(ft: FILETIME) -> u64 {
        ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
    }

    fn sample_raw(&mut self) -> (u64, u64, u64) {
        let mut idle = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();

        unsafe {
            let _ = GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user));
        }

        let idle_val = Self::filetime_to_u64(idle);
        let kernel_val = Self::filetime_to_u64(kernel);
        let user_val = Self::filetime_to_u64(user);

        self.last_idle = idle_val;
        self.last_kernel = kernel_val;
        self.last_user = user_val;

        (idle_val, kernel_val, user_val)
    }

    fn refresh_metrics(&mut self) {
        let prev_idle = self.last_idle;
        let prev_kernel = self.last_kernel;
        let prev_user = self.last_user;

        let (cur_idle, cur_kernel, cur_user) = self.sample_raw();

        // Calculate CPU delta
        let idle_delta = cur_idle.saturating_sub(prev_idle);
        let kernel_delta = cur_kernel.saturating_sub(prev_kernel);
        let user_delta = cur_user.saturating_sub(prev_user);

        let total_system = kernel_delta + user_delta;
        let cpu_pct = if total_system > 0 && total_system >= idle_delta {
            let busy = total_system - idle_delta;
            ((busy as f64 / total_system as f64) * 100.0) as f32
        } else {
            0.0
        };

        // Query RAM
        let mut mem = MEMORYSTATUSEX::default();
        mem.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

        let (ram_pct, ram_used_gb, ram_total_gb) = unsafe {
            if GlobalMemoryStatusEx(&mut mem).is_ok() {
                let total = mem.ullTotalPhys as f64 / (1024.0 * 1024.0 * 1024.0);
                let avail = mem.ullAvailPhys as f64 / (1024.0 * 1024.0 * 1024.0);
                let used = (total - avail).max(0.0);
                (mem.dwMemoryLoad, used as f32, total as f32)
            } else {
                (0, 0.0, 0.0)
            }
        };

        self.cached_metrics = SystemMetrics {
            cpu_usage_pct: cpu_pct.clamp(0.0, 100.0),
            ram_usage_pct: ram_pct,
            ram_used_gb,
            ram_total_gb,
        };

        self.last_sample_time = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_monitor_query() {
        let mut monitor = SystemMonitor::new();
        let metrics = monitor.get_metrics();
        // RAM total should be positive on any real machine
        assert!(metrics.ram_total_gb > 0.0);
        assert!(metrics.ram_usage_pct <= 100);
        assert!(metrics.cpu_usage_pct >= 0.0 && metrics.cpu_usage_pct <= 100.0);
    }

    #[test]
    fn test_filetime_conversion() {
        let ft = FILETIME {
            dwLowDateTime: 0x00000005,
            dwHighDateTime: 0x00000001,
        };
        let val = SystemMonitor::filetime_to_u64(ft);
        assert_eq!(val, 0x00000001_00000005);
    }
}
