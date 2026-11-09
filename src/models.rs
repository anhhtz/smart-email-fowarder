use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub host: String,
    pub port: u16,
    pub security: String, // "ssl", "starttls", "plain"
    pub username: String,
    pub password: Option<String>,
    #[serde(default = "default_folder")]
    pub folder: String,
    #[serde(default = "default_true")]
    pub mark_as_read: bool,
    #[serde(default = "default_false")]
    pub allow_self_signed: bool,
}

fn default_folder() -> String {
    "INBOX".to_string()
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardingConfig {
    pub target_emails: Vec<String>,
    #[serde(default)]
    pub subject_prefix: String,
    #[serde(default = "default_interval")]
    pub poll_interval_minutes: u64,
    #[serde(default = "default_true")]
    pub include_banner: bool,
    #[serde(default = "default_true")]
    pub preserve_reply_to: bool,
}

fn default_interval() -> u64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub security: String, // "plain", "starttls", "ssl"
    pub from_email: String,
    #[serde(default = "default_from_name")]
    pub from_name: String,
    #[serde(default = "default_false")]
    pub auth_required: bool,
    #[serde(default)]
    pub username: String,
    pub password: Option<String>,
}

fn default_from_name() -> String {
    "Exchange Email Forwarder".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub exchange: Option<ExchangeConfig>,
    pub forwarding: Option<ForwardingConfig>,
    pub smtp: Option<SmtpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardLog {
    pub id: i64,
    pub timestamp: String,
    pub message_id: String,
    pub original_sender: String,
    pub subject: String,
    pub forwarded_to: String,
    pub status: String, // "SUCCESS" or "FAILED"
    pub error_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLogEntry {
    pub id: i64,
    pub time: String,
    pub level: String, // "INFO", "WARN", "ERROR", "SUCCESS"
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub is_polling: bool,
    pub is_busy: bool,
    pub poll_interval_minutes: u64,
    pub last_poll_time: Option<String>,
    pub next_poll_seconds: Option<i64>,
    pub total_forwarded: u64,
    pub total_failed: u64,
}

#[derive(Debug, Serialize)]
pub struct ExchangeTestResult {
    pub message_count: u32,
    pub unseen_count: u32,
}

#[derive(Debug, Serialize)]
pub struct SmtpTestResult {
    pub recipient: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub forwarded_count: usize,
    pub failed_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMemoryInfo {
    pub rss_bytes: Option<u64>,
    pub rss_formatted: String,
    pub vmsize_bytes: Option<u64>,
    pub vmsize_formatted: String,
    pub vmpeak_bytes: Option<u64>,
    pub vmpeak_formatted: String,
    pub thread_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    pub path: String,
    pub size_bytes: u64,
    pub size_formatted: String,
    pub total_processed_emails: u64,
    pub total_forward_logs: u64,
    pub total_system_logs: u64,
    pub integrity_status: String,
    pub journal_mode: String,
    pub is_healthy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemHealthCheck {
    pub name: String,
    pub status: String, // "UP", "WARN", "DOWN"
    pub latency_ms: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHealthReport {
    pub overall_status: String, // "HEALTHY", "WARNING", "DEGRADED"
    pub checked_at: String,
    pub checks: Vec<SubsystemHealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStatus {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub service_mode: String, // "Interactive Console", "Windows Service", "systemd Daemon"
    pub status: String,
    pub started_at: String,
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub exe_path: String,
    pub current_dir: String,
    pub cmd_args: Vec<String>,
    pub os: String,
    pub arch: String,
    pub cpu_cores: usize,
    pub load_average: Option<Vec<f64>>,
    pub memory: ProcessMemoryInfo,
    pub database: DatabaseMetrics,
    pub poller: SystemStatus,
    pub app_version: String,
}

