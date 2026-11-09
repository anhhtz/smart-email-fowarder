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
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,
    #[serde(default = "default_smtp_security")]
    pub smtp_security: String,
    #[serde(default)]
    pub from_email: Option<String>,
    #[serde(default = "default_from_name")]
    pub from_name: String,
}

fn default_smtp_port() -> u16 {
    587
}
fn default_smtp_security() -> String {
    "starttls".to_string()
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

impl ExchangeConfig {
    pub fn to_smtp_config(&self) -> SmtpConfig {
        let from_email = self
            .from_email
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&self.username)
            .to_string();

        let from_name = if self.from_name.trim().is_empty() {
            "Exchange Email Forwarder".to_string()
        } else {
            self.from_name.clone()
        };

        SmtpConfig {
            host: self.host.clone(),
            port: self.smtp_port,
            security: self.smtp_security.clone(),
            from_email,
            from_name,
            auth_required: true,
            use_exchange_credentials: true,
            allow_self_signed: self.allow_self_signed,
            username: self.username.clone(),
            password: self.password.clone(),
        }
    }
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
    #[serde(default = "default_false")]
    pub use_exchange_credentials: bool,
    #[serde(default = "default_false")]
    pub allow_self_signed: bool,
    #[serde(default)]
    pub username: String,
    pub password: Option<String>,
}

fn default_from_name() -> String {
    "Exchange Email Forwarder".to_string()
}

impl SmtpConfig {
    pub fn resolve_with_exchange(&self, exchange: Option<&ExchangeConfig>) -> SmtpConfig {
        let mut resolved = self.clone();
        if resolved.use_exchange_credentials {
            resolved.auth_required = true;
            if let Some(ex) = exchange {
                resolved.username = ex.username.clone();
                resolved.password = ex.password.clone();
                resolved.allow_self_signed = ex.allow_self_signed;
            } else {
                resolved.username = String::new();
                resolved.password = None;
            }
        }
        resolved
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxMessageSummary {
    pub uid: u32,
    pub message_id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: String,
    pub is_seen: bool,
    pub is_forwarded: bool,
    pub has_attachments: bool,
    pub size_formatted: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxAttachmentInfo {
    pub index: usize,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: usize,
    pub size_formatted: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxMessageDetail {
    pub uid: u32,
    pub message_id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub cc: String,
    pub date: String,
    pub is_seen: bool,
    pub is_forwarded: bool,
    pub body_text: String,
    pub body_html: Option<String>,
    pub raw_headers: String,
    pub attachments: Vec<MailboxAttachmentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxFolderList {
    pub current_folder: String,
    pub folders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxListResponse {
    pub folder: String,
    pub total: usize,
    pub page: usize,
    pub limit: usize,
    pub total_pages: usize,
    pub messages: Vec<MailboxMessageSummary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MailboxListQuery {
    pub folder: Option<String>,
    pub filter: Option<String>, // "all", "unseen", "seen"
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub q: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MailboxMessageQuery {
    pub folder: Option<String>,
    pub uid: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MailboxAttachmentQuery {
    pub folder: Option<String>,
    pub uid: u32,
    pub index: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MailboxActionRequest {
    pub folder: Option<String>,
    pub uid: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MailboxMarkReadRequest {
    pub folder: Option<String>,
    pub uid: u32,
    pub seen: bool,
}

// Authentication Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub token: String,
    pub username: String,
    pub provider: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    #[serde(default)]
    pub provider: Option<String>, // "exchange" or "google"
    pub username: String,
    pub password: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub security: Option<String>,
    pub allow_self_signed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatusResponse {
    pub authenticated: bool,
    pub user: Option<UserSession>,
    pub exchange_configured: bool,
    #[serde(default)]
    pub google_configured: bool,
    #[serde(default)]
    pub default_provider: Option<String>,
    pub default_host: Option<String>,
    pub default_username: Option<String>,
    #[serde(default)]
    pub default_allow_self_signed: bool,
    #[serde(default)]
    pub default_port: Option<u16>,
    #[serde(default)]
    pub default_security: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProviderInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_active: bool,
    pub stage: String, // "active" or "next_stage"
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProvidersResponse {
    pub providers: Vec<AuthProviderInfo>,
    pub exchange_configured: bool,
}

// Google OAuth2 Models (Slot for Next Stage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleOAuthConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default = "default_google_redirect_uri")]
    pub redirect_uri: String,
    #[serde(default = "default_google_scopes")]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub auth_url_preview: Option<String>,
}

fn default_google_redirect_uri() -> String {
    "http://localhost:8080/api/auth/oauth/google/callback".to_string()
}

fn default_google_scopes() -> Vec<String> {
    vec![
        "openid".to_string(),
        "email".to_string(),
        "profile".to_string(),
    ]
}

impl Default for GoogleOAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            client_id: None,
            client_secret: None,
            redirect_uri: default_google_redirect_uri(),
            scopes: default_google_scopes(),
            auth_url_preview: None,
        }
    }
}


