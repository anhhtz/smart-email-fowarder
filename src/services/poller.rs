use crate::db::Database;
use crate::models::{SyncResult, SystemStatus};
use crate::services::exchange::ExchangeClient;
use crate::services::forwarder::Forwarder;
use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time::sleep;
use tracing::{error, info};

pub struct PollerManager {
    db: Database,
    is_polling: Arc<AtomicBool>,
    is_busy: Arc<AtomicBool>,
    last_poll_time: Arc<Mutex<Option<DateTime<Utc>>>>,
    next_poll_time: Arc<Mutex<Option<DateTime<Utc>>>>,
    sync_trigger_tx: mpsc::Sender<()>,
}

impl PollerManager {
    pub fn new(db: Database) -> (Self, mpsc::Receiver<()>) {
        let (tx, rx) = mpsc::channel(1);
        let manager = Self {
            db,
            is_polling: Arc::new(AtomicBool::new(true)),
            is_busy: Arc::new(AtomicBool::new(false)),
            last_poll_time: Arc::new(Mutex::new(None)),
            next_poll_time: Arc::new(Mutex::new(None)),
            sync_trigger_tx: tx,
        };
        (manager, rx)
    }

    pub fn pause(&self) {
        self.is_polling.store(false, Ordering::SeqCst);
        let _ = self.db.log_event("WARN", "Background poller paused by user.");
    }

    pub fn resume(&self) {
        self.is_polling.store(true, Ordering::SeqCst);
        let _ = self.db.log_event("INFO", "Background poller resumed.");
    }

    pub async fn trigger_sync(&self) -> Result<SyncResult, String> {
        let _ = self.sync_trigger_tx.try_send(());
        Self::execute_sync_cycle(&self.db, &self.is_busy, &self.last_poll_time).await
    }

    pub async fn get_status(&self) -> SystemStatus {
        let is_polling = self.is_polling.load(Ordering::SeqCst);
        let is_busy = self.is_busy.load(Ordering::SeqCst);

        let cfg = self.db.get_config().unwrap_or_default();
        let interval_min = cfg
            .forwarding
            .as_ref()
            .map(|f| f.poll_interval_minutes)
            .unwrap_or(1);

        let last_time_str = self
            .last_poll_time
            .lock()
            .await
            .map(|t| t.to_rfc3339());

        let next_seconds = if !is_polling {
            None
        } else {
            let next_guard = self.next_poll_time.lock().await;
            next_guard.map(|nt| {
                let diff = (nt - Utc::now()).num_seconds();
                if diff < 0 {
                    0
                } else {
                    diff
                }
            })
        };

        let (total_forwarded, total_failed) = self.db.get_stats().unwrap_or((0, 0));

        SystemStatus {
            is_polling,
            is_busy,
            poll_interval_minutes: interval_min,
            last_poll_time: last_time_str,
            next_poll_seconds: next_seconds,
            total_forwarded,
            total_failed,
        }
    }

    pub fn start_background_loop(
        &self,
        mut sync_trigger_rx: mpsc::Receiver<()>,
    ) {
        let db = self.db.clone();
        let is_polling = Arc::clone(&self.is_polling);
        let is_busy = Arc::clone(&self.is_busy);
        let last_poll = Arc::clone(&self.last_poll_time);
        let next_poll = Arc::clone(&self.next_poll_time);

        tokio::spawn(async move {
            info!("Background poller task started.");
            let _ = db.log_event("INFO", "Background poller task started.");

            loop {
                // Determine interval
                let interval_sec = {
                    let cfg = db.get_config().unwrap_or_default();
                    let mins = cfg
                        .forwarding
                        .as_ref()
                        .map(|f| f.poll_interval_minutes)
                        .unwrap_or(1)
                        .max(1);
                    mins * 60
                };

                let next = Utc::now() + chrono::Duration::seconds(interval_sec as i64);
                {
                    let mut np = next_poll.lock().await;
                    *np = Some(next);
                }

                tokio::select! {
                    _ = sleep(Duration::from_secs(interval_sec)) => {
                        if is_polling.load(Ordering::SeqCst) {
                            let _ = Self::execute_sync_cycle(&db, &is_busy, &last_poll).await;
                        }
                    }
                    Some(_) = sync_trigger_rx.recv() => {
                        info!("Manual sync triggered via channel.");
                        // Already executed by trigger_sync() or execute here
                    }
                }
            }
        });
    }

    async fn execute_sync_cycle(
        db: &Database,
        is_busy: &Arc<AtomicBool>,
        last_poll_time: &Arc<Mutex<Option<DateTime<Utc>>>>,
    ) -> Result<SyncResult, String> {
        // Prevent concurrent sync executions
        if is_busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("A sync cycle is already in progress".to_string());
        }

        let run_result = Self::do_sync(db).await;

        {
            let mut lpt = last_poll_time.lock().await;
            *lpt = Some(Utc::now());
        }

        is_busy.store(false, Ordering::SeqCst);
        run_result
    }

    async fn do_sync(db: &Database) -> Result<SyncResult, String> {
        let cfg = db
            .get_config()
            .map_err(|e| format!("Database config read error: {}", e))?;

        let exchange_cfg = match cfg.exchange {
            Some(e) => e,
            None => {
                let msg = "Exchange settings not configured. Skipping poll cycle.";
                let _ = db.log_event("WARN", msg);
                return Ok(SyncResult { forwarded_count: 0, failed_count: 0 });
            }
        };

        let forward_cfg = match cfg.forwarding {
            Some(f) => f,
            None => {
                let msg = "Forwarding rules not configured. Skipping poll cycle.";
                let _ = db.log_event("WARN", msg);
                return Ok(SyncResult { forwarded_count: 0, failed_count: 0 });
            }
        };

        let smtp_cfg = match cfg.smtp {
            Some(s) => s,
            None => {
                let msg = "Exchange SMTP relay not configured. Skipping poll cycle.";
                let _ = db.log_event("WARN", msg);
                return Ok(SyncResult { forwarded_count: 0, failed_count: 0 });
            }
        };

        if forward_cfg.target_emails.is_empty() {
            let msg = "No target Gmail addresses configured. Skipping poll cycle.";
            let _ = db.log_event("WARN", msg);
            return Ok(SyncResult { forwarded_count: 0, failed_count: 0 });
        }

        let _ = db.log_event("INFO", &format!("Checking Exchange mailbox '{}'...", exchange_cfg.folder));

        // Ingest emails from Exchange IMAP
        let emails = match ExchangeClient::fetch_unseen_emails(&exchange_cfg).await {
            Ok(list) => list,
            Err(e) => {
                let err_msg = format!("Failed to poll Exchange mailbox: {}", e);
                error!("{}", err_msg);
                let _ = db.log_event("ERROR", &err_msg);
                return Err(err_msg);
            }
        };

        if emails.is_empty() {
            return Ok(SyncResult { forwarded_count: 0, failed_count: 0 });
        }

        let mut forwarded_count = 0;
        let mut failed_count = 0;

        for email in emails {
            let parsed_summary = mail_parser::MessageParser::default().parse(&email.raw_bytes);
            let msg_id = parsed_summary
                .as_ref()
                .and_then(|p| p.message_id())
                .unwrap_or("unknown")
                .to_string();

            let _subject = parsed_summary
                .as_ref()
                .and_then(|p| p.subject())
                .unwrap_or("(No Subject)")
                .to_string();

            // Check deduplication
            if let Ok(true) = db.is_processed(&msg_id, email.uid) {
                continue;
            }

            let result = Forwarder::forward_email(&email.raw_bytes, &forward_cfg, &smtp_cfg).await;

            if result.success {
                forwarded_count += 1;
                let _ = db.mark_processed(&result.message_id, email.uid, &result.subject);
                let _ = db.record_forward_log(
                    &result.message_id,
                    &result.original_sender,
                    &result.subject,
                    &result.forwarded_to,
                    "SUCCESS",
                    None,
                );

                let _ = db.log_event(
                    "SUCCESS",
                    &format!(
                        "Forwarded: '{}' from {} -> {}",
                        result.subject, result.original_sender, result.forwarded_to
                    ),
                );

                if exchange_cfg.mark_as_read {
                    let _ = ExchangeClient::mark_as_read(&exchange_cfg, email.uid).await;
                }
            } else {
                failed_count += 1;
                let err_text = result.error.as_deref().unwrap_or("Unknown error");
                let _ = db.record_forward_log(
                    &result.message_id,
                    &result.original_sender,
                    &result.subject,
                    &result.forwarded_to,
                    "FAILED",
                    Some(err_text),
                );

                let _ = db.log_event(
                    "ERROR",
                    &format!(
                        "Failed forwarding '{}' to {}: {}",
                        result.subject, result.forwarded_to, err_text
                    ),
                );
            }
        }

        Ok(SyncResult {
            forwarded_count,
            failed_count,
        })
    }
}
