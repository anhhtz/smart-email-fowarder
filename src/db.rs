use crate::models::{
    AppConfig, DatabaseMetrics, ExchangeConfig, ForwardLog, ForwardingConfig, GoogleOAuthConfig,
    SmtpConfig, SystemLogEntry, UserSession,
};
use chrono::Utc;
use rusqlite::{params, Connection, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::info;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: Option<PathBuf>,
}

fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    for b in bytes.iter_mut() {
        *b = fastrand::u8(..);
    }
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(path_ref)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: Some(path_ref.to_path_buf()),
        };
        db.init_tables()?;
        Ok(db)
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: None,
        };
        db.init_tables()?;
        Ok(db)
    }

    pub fn get_path(&self) -> Option<PathBuf> {
        self.path.clone()
    }

    pub fn get_db_metrics(&self) -> Result<DatabaseMetrics> {
        let conn = self.conn.lock().unwrap();

        let path_str = self
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ":memory:".to_string());

        let size_bytes = if let Some(ref p) = self.path {
            std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        let total_processed_emails: u64 = conn
            .query_row("SELECT COUNT(*) FROM processed_emails;", [], |r| r.get(0))
            .unwrap_or(0);

        let total_forward_logs: u64 = conn
            .query_row("SELECT COUNT(*) FROM forward_logs;", [], |r| r.get(0))
            .unwrap_or(0);

        let total_system_logs: u64 = conn
            .query_row("SELECT COUNT(*) FROM system_logs;", [], |r| r.get(0))
            .unwrap_or(0);

        let integrity_status: String = conn
            .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
            .unwrap_or_else(|_| "unknown".to_string());

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
            .unwrap_or_else(|_| "unknown".to_string());

        let is_healthy = integrity_status == "ok";
        let size_formatted = crate::services::process::format_bytes(size_bytes);

        Ok(DatabaseMetrics {
            path: path_str,
            size_bytes,
            size_formatted,
            total_processed_emails,
            total_forward_logs,
            total_system_logs,
            integrity_status,
            journal_mode: journal_mode.to_uppercase(),
            is_healthy,
        })
    }

    pub fn ping(&self) -> Result<f64> {
        let start = std::time::Instant::now();
        let conn = self.conn.lock().unwrap();
        let _: i64 = conn.query_row("SELECT 1;", [], |r| r.get(0))?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        Ok(elapsed_ms)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Settings Key-Value Table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
            [],
        )?;

        // Processed Emails Table (Deduplication)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS processed_emails (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT NOT NULL,
                uid INTEGER NOT NULL,
                subject TEXT,
                processed_at TEXT NOT NULL,
                UNIQUE(message_id, uid)
            );",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_processed_msgid ON processed_emails (message_id);",
            [],
        )?;

        // Forward Logs Table (History for UI)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS forward_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                message_id TEXT NOT NULL,
                original_sender TEXT NOT NULL,
                subject TEXT NOT NULL,
                forwarded_to TEXT NOT NULL,
                status TEXT NOT NULL,
                error_details TEXT
            );",
            [],
        )?;

        // System Diagnostic Logs
        conn.execute(
            "CREATE TABLE IF NOT EXISTS system_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                time TEXT NOT NULL,
                level TEXT NOT NULL,
                message TEXT NOT NULL
            );",
            [],
        )?;

        // Auth Sessions Table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS auth_sessions (
                token TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                provider TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            );",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_expires ON auth_sessions (expires_at);",
            [],
        )?;

        info!("SQLite tables initialized successfully.");
        Ok(())
    }

    // Config Methods
    pub fn get_config(&self) -> Result<AppConfig> {
        let conn = self.conn.lock().unwrap();
        let mut cfg = AppConfig::default();

        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let val: String = row.get(1)?;
            Ok((key, val))
        })?;

        for row in rows {
            let (key, val) = row?;
            match key.as_str() {
                "exchange" => {
                    if let Ok(c) = serde_json::from_str::<ExchangeConfig>(&val) {
                        cfg.exchange = Some(c);
                    }
                }
                "forwarding" => {
                    if let Ok(c) = serde_json::from_str::<ForwardingConfig>(&val) {
                        cfg.forwarding = Some(c);
                    }
                }
                "smtp" => {
                    if let Ok(c) = serde_json::from_str::<SmtpConfig>(&val) {
                        cfg.smtp = Some(c);
                    }
                }
                _ => {}
            }
        }

        Ok(cfg)
    }

    pub fn save_exchange_config(&self, new_cfg: &mut ExchangeConfig) -> Result<()> {
        let existing = self.get_config()?.exchange;

        // Preserve existing password if user left it blank in UI
        if new_cfg.password.as_deref().unwrap_or("").trim().is_empty() {
            if let Some(old) = existing {
                new_cfg.password = old.password;
            }
        }

        let json = serde_json::to_string(new_cfg).unwrap_or_default();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('exchange', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1;",
            params![json],
        )?;

        Ok(())
    }

    pub fn save_forwarding_config(&self, cfg: &ForwardingConfig) -> Result<()> {
        let json = serde_json::to_string(cfg).unwrap_or_default();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('forwarding', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1;",
            params![json],
        )?;

        Ok(())
    }

    pub fn save_smtp_config(&self, new_cfg: &mut SmtpConfig) -> Result<()> {
        let existing = self.get_config()?.smtp;

        // Preserve existing password if user left it blank in UI (and not using exchange credentials)
        if !new_cfg.use_exchange_credentials && new_cfg.password.as_deref().unwrap_or("").trim().is_empty() {
            if let Some(old) = existing {
                new_cfg.password = old.password;
            }
        }

        let json = serde_json::to_string(new_cfg).unwrap_or_default();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('smtp', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1;",
            params![json],
        )?;

        Ok(())
    }

    // Deduplication Methods
    pub fn is_processed(&self, message_id: &str, uid: u32) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM processed_emails WHERE message_id = ?1 OR uid = ?2;",
            params![message_id, uid],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn mark_processed(&self, message_id: &str, uid: u32, subject: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO processed_emails (message_id, uid, subject, processed_at) VALUES (?1, ?2, ?3, ?4);",
            params![message_id, uid, subject, now],
        )?;
        Ok(())
    }

    // Forward Logs Methods
    pub fn record_forward_log(
        &self,
        message_id: &str,
        sender: &str,
        subject: &str,
        forwarded_to: &str,
        status: &str,
        error_details: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO forward_logs (timestamp, message_id, original_sender, subject, forwarded_to, status, error_details)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            params![now, message_id, sender, subject, forwarded_to, status, error_details],
        )?;
        Ok(())
    }

    pub fn get_forward_logs(&self, limit: usize) -> Result<Vec<ForwardLog>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, message_id, original_sender, subject, forwarded_to, status, error_details
             FROM forward_logs ORDER BY id DESC LIMIT ?1;",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(ForwardLog {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                message_id: row.get(2)?,
                original_sender: row.get(3)?,
                subject: row.get(4)?,
                forwarded_to: row.get(5)?,
                status: row.get(6)?,
                error_details: row.get(7)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_stats(&self) -> Result<(u64, u64)> {
        let conn = self.conn.lock().unwrap();
        let success: u64 = conn.query_row(
            "SELECT COUNT(*) FROM forward_logs WHERE status = 'SUCCESS';",
            [],
            |r| r.get(0),
        ).unwrap_or(0);

        let failed: u64 = conn.query_row(
            "SELECT COUNT(*) FROM forward_logs WHERE status != 'SUCCESS';",
            [],
            |r| r.get(0),
        ).unwrap_or(0);

        Ok((success, failed))
    }

    // System Logging Methods
    pub fn log_event(&self, level: &str, message: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO system_logs (time, level, message) VALUES (?1, ?2, ?3);",
            params![now, level, message],
        )?;

        // Prune older logs beyond 1000 entries
        let _ = conn.execute(
            "DELETE FROM system_logs WHERE id NOT IN (SELECT id FROM system_logs ORDER BY id DESC LIMIT 1000);",
            [],
        );

        Ok(())
    }

    pub fn get_recent_system_logs(&self, limit: usize) -> Result<Vec<SystemLogEntry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, time, level, message FROM system_logs ORDER BY id DESC LIMIT ?1;",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(SystemLogEntry {
                id: row.get(0)?,
                time: row.get(1)?,
                level: row.get(2)?,
                message: row.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        list.reverse();
        Ok(list)
    }

    // Authentication & Session Methods
    pub fn create_auth_session(
        &self,
        username: &str,
        provider: &str,
        duration_hours: i64,
    ) -> Result<UserSession> {
        let token = generate_session_token();
        let now = Utc::now();
        let expires = now + chrono::Duration::hours(duration_hours);

        let session = UserSession {
            token: token.clone(),
            username: username.to_string(),
            provider: provider.to_string(),
            created_at: now.to_rfc3339(),
            expires_at: expires.to_rfc3339(),
        };

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO auth_sessions (token, username, provider, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5);",
            params![
                session.token,
                session.username,
                session.provider,
                session.created_at,
                session.expires_at,
            ],
        )?;

        Ok(session)
    }

    pub fn validate_auth_session(&self, token: &str) -> Result<Option<UserSession>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT token, username, provider, created_at, expires_at FROM auth_sessions WHERE token = ?1;",
        )?;

        let mut rows = stmt.query(params![token])?;
        if let Some(row) = rows.next()? {
            let token: String = row.get(0)?;
            let username: String = row.get(1)?;
            let provider: String = row.get(2)?;
            let created_at: String = row.get(3)?;
            let expires_at_str: String = row.get(4)?;

            if let Ok(expires_at) = chrono::DateTime::parse_from_rfc3339(&expires_at_str) {
                if expires_at.with_timezone(&Utc) < Utc::now() {
                    drop(rows);
                    drop(stmt);
                    let _ = conn.execute("DELETE FROM auth_sessions WHERE token = ?1;", params![token]);
                    return Ok(None);
                }
            }

            Ok(Some(UserSession {
                token,
                username,
                provider,
                created_at,
                expires_at: expires_at_str,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_auth_session(&self, token: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM auth_sessions WHERE token = ?1;", params![token])?;
        Ok(())
    }

    pub fn cleanup_expired_sessions(&self) -> Result<usize> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        let count = conn.execute("DELETE FROM auth_sessions WHERE expires_at < ?1;", params![now])?;
        Ok(count)
    }

    pub fn get_active_session_count(&self) -> Result<usize> {
        let now = Utc::now().to_rfc3339();
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM auth_sessions WHERE expires_at >= ?1;",
            params![now],
            |r| r.get(0),
        )?;
        Ok(count as usize)
    }

    // Google OAuth2 Config Slot Methods (Stage 2)
    pub fn get_google_oauth_config(&self) -> Result<GoogleOAuthConfig> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = 'google_oauth';")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let val: String = row.get(0)?;
            if let Ok(cfg) = serde_json::from_str::<GoogleOAuthConfig>(&val) {
                return Ok(cfg);
            }
        }
        Ok(GoogleOAuthConfig::default())
    }

    pub fn save_google_oauth_config(&self, cfg: &GoogleOAuthConfig) -> Result<()> {
        let json = serde_json::to_string(cfg).unwrap_or_default();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('google_oauth', ?1)
             ON CONFLICT(key) DO UPDATE SET value = ?1;",
            params![json],
        )?;
        Ok(())
    }
}
