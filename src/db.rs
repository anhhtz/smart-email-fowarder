use crate::models::{AppConfig, ExchangeConfig, ForwardLog, ForwardingConfig, SmtpConfig, SystemLogEntry};
use chrono::Utc;
use rusqlite::{params, Connection, Result};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::info;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_tables()?;
        Ok(db)
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_tables()?;
        Ok(db)
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

        // Preserve existing password if user left it blank in UI
        if new_cfg.password.as_deref().unwrap_or("").trim().is_empty() {
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
}
