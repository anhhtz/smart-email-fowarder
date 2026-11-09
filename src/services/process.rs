use crate::db::Database;
use crate::models::{
    DatabaseMetrics, ProcessHealthReport, ProcessMemoryInfo, ProcessStatus, SubsystemHealthCheck,
    SystemStatus,
};
use crate::services::poller::PollerManager;
use chrono::{DateTime, Utc};
use std::time::Instant;

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

pub fn detect_service_mode() -> &'static str {
    #[cfg(windows)]
    {
        if std::env::args().any(|arg| arg == "--service") {
            return "Windows Service";
        }
    }
    if std::env::var("INVOCATION_ID").is_ok() || std::env::var("JOURNAL_STREAM").is_ok() {
        return "systemd Service";
    }
    "Interactive Console"
}

#[cfg(target_os = "linux")]
fn parse_kb_to_bytes(s: &str) -> Option<u64> {
    let trimmed = s.trim().trim_end_matches("kB").trim();
    trimmed.parse::<u64>().ok().map(|kb| kb * 1024)
}

pub fn get_process_memory_info() -> ProcessMemoryInfo {
    let mut rss: Option<u64> = None;
    let mut vmsize: Option<u64> = None;
    let mut vmpeak: Option<u64> = None;
    let mut threads: Option<usize> = None;

    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("VmRSS:") {
                    rss = parse_kb_to_bytes(rest);
                } else if let Some(rest) = line.strip_prefix("VmSize:") {
                    vmsize = parse_kb_to_bytes(rest);
                } else if let Some(rest) = line.strip_prefix("VmPeak:") {
                    vmpeak = parse_kb_to_bytes(rest);
                } else if let Some(rest) = line.strip_prefix("Threads:") {
                    threads = rest.trim().parse::<usize>().ok();
                }
            }
        }
    }

    ProcessMemoryInfo {
        rss_bytes: rss,
        rss_formatted: rss.map(format_bytes).unwrap_or_else(|| "N/A".to_string()),
        vmsize_bytes: vmsize,
        vmsize_formatted: vmsize.map(format_bytes).unwrap_or_else(|| "N/A".to_string()),
        vmpeak_bytes: vmpeak,
        vmpeak_formatted: vmpeak.map(format_bytes).unwrap_or_else(|| "N/A".to_string()),
        thread_count: threads,
    }
}

pub fn get_parent_pid() -> Option<u32> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("PPid:") {
                    return rest.trim().parse::<u32>().ok();
                }
            }
        }
    }
    None
}

pub fn get_load_average() -> Option<Vec<f64>> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/loadavg") {
            let parts: Vec<f64> = content
                .split_whitespace()
                .take(3)
                .filter_map(|s| s.parse::<f64>().ok())
                .collect();
            if parts.len() == 3 {
                return Some(parts);
            }
        }
    }
    None
}

pub fn collect_process_status(
    db: &Database,
    poller_status: SystemStatus,
    start_time: DateTime<Utc>,
) -> ProcessStatus {
    let pid = std::process::id();
    let ppid = get_parent_pid();
    let service_mode = detect_service_mode().to_string();
    let started_at = start_time.to_rfc3339();
    let uptime_seconds = (Utc::now() - start_time).num_seconds().max(0) as u64;
    let uptime_formatted = format_duration(uptime_seconds);

    let exe_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let current_dir = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let cmd_args: Vec<String> = std::env::args().collect();
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let load_average = get_load_average();
    let memory = get_process_memory_info();

    let database = db.get_db_metrics().unwrap_or(DatabaseMetrics {
        path: "unknown".to_string(),
        size_bytes: 0,
        size_formatted: "0 B".to_string(),
        total_processed_emails: 0,
        total_forward_logs: 0,
        total_system_logs: 0,
        integrity_status: "error".to_string(),
        journal_mode: "unknown".to_string(),
        is_healthy: false,
    });

    ProcessStatus {
        pid,
        ppid,
        service_mode,
        status: "Running".to_string(),
        started_at,
        uptime_seconds,
        uptime_formatted,
        exe_path,
        current_dir,
        cmd_args,
        os,
        arch,
        cpu_cores,
        load_average,
        memory,
        database,
        poller: poller_status,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub async fn collect_health_report(
    db: &Database,
    poller: &PollerManager,
) -> ProcessHealthReport {
    let mut checks = Vec::new();

    // 1. Tokio Async Runtime Check
    let rt_start = Instant::now();
    tokio::task::yield_now().await;
    let rt_latency = (rt_start.elapsed().as_secs_f64() * 1000.0 * 100.0).round() / 100.0;
    checks.push(SubsystemHealthCheck {
        name: "Tokio Async Runtime".to_string(),
        status: "UP".to_string(),
        latency_ms: rt_latency,
        message: "Async executor active and scheduling tasks normally".to_string(),
    });

    // 2. SQLite Database Engine Check
    match db.ping() {
        Ok(lat) => {
            let lat_rounded = (lat * 100.0).round() / 100.0;
            let metrics = db.get_db_metrics().ok();
            let integrity = metrics
                .as_ref()
                .map(|m| m.integrity_status.as_str())
                .unwrap_or("ok");
            let is_ok = integrity == "ok";
            checks.push(SubsystemHealthCheck {
                name: "SQLite Storage Engine".to_string(),
                status: if is_ok { "UP" } else { "WARN" }.to_string(),
                latency_ms: lat_rounded,
                message: format!(
                    "Integrity: {}, Journal: {}",
                    integrity,
                    metrics.as_ref().map(|m| m.journal_mode.as_str()).unwrap_or("WAL")
                ),
            });
        }
        Err(e) => {
            checks.push(SubsystemHealthCheck {
                name: "SQLite Storage Engine".to_string(),
                status: "DOWN".to_string(),
                latency_ms: 0.0,
                message: format!("Database query error: {}", e),
            });
        }
    }

    // 3. Background Poller Worker Check
    let poller_start = Instant::now();
    let poller_status = poller.get_status().await;
    let poller_latency = (poller_start.elapsed().as_secs_f64() * 1000.0 * 100.0).round() / 100.0;
    let poller_state_str = if poller_status.is_polling {
        if poller_status.is_busy {
            "Sync cycle currently active"
        } else {
            "Poller active and waiting for next scheduled interval"
        }
    } else {
        "Poller is currently paused by operator"
    };
    checks.push(SubsystemHealthCheck {
        name: "Background Poller Worker".to_string(),
        status: if poller_status.is_polling { "UP" } else { "WARN" }.to_string(),
        latency_ms: poller_latency,
        message: format!(
            "{} (interval: {}m, next: {}s)",
            poller_state_str,
            poller_status.poll_interval_minutes,
            poller_status.next_poll_seconds.unwrap_or(0)
        ),
    });

    // 4. Inbound Exchange IMAP Client Configuration
    let cfg = db.get_config().unwrap_or_default();
    if let Some(ref ex) = cfg.exchange {
        checks.push(SubsystemHealthCheck {
            name: "Exchange IMAP Connection".to_string(),
            status: "UP".to_string(),
            latency_ms: 0.0,
            message: format!(
                "Configured: {}:{} ({}), Folder: '{}'",
                ex.host, ex.port, ex.security, ex.folder
            ),
        });
    } else {
        checks.push(SubsystemHealthCheck {
            name: "Exchange IMAP Connection".to_string(),
            status: "WARN".to_string(),
            latency_ms: 0.0,
            message: "Mailbox connection not yet configured in settings".to_string(),
        });
    }

    // 5. Outbound Exchange SMTP Relay Configuration
    if let Some(ref sm) = cfg.smtp {
        let auth_desc = if sm.auth_required { "Auth: Yes" } else { "Auth: No" };
        checks.push(SubsystemHealthCheck {
            name: "Outbound SMTP Relay".to_string(),
            status: "UP".to_string(),
            latency_ms: 0.0,
            message: format!(
                "Configured: {}:{} ({}, {}), Sender: '{}'",
                sm.host, sm.port, sm.security, auth_desc, sm.from_email
            ),
        });
    } else {
        checks.push(SubsystemHealthCheck {
            name: "Outbound SMTP Relay".to_string(),
            status: "WARN".to_string(),
            latency_ms: 0.0,
            message: "SMTP relay parameters not yet configured in settings".to_string(),
        });
    }

    let overall_status = if checks.iter().any(|c| c.status == "DOWN") {
        "DEGRADED".to_string()
    } else if checks.iter().any(|c| c.status == "WARN") {
        "WARNING".to_string()
    } else {
        "HEALTHY".to_string()
    };

    ProcessHealthReport {
        overall_status,
        checked_at: Utc::now().to_rfc3339(),
        checks,
    }
}

