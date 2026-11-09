use crate::models::{ExchangeConfig, ExchangeTestResult};
use futures_util::TryStreamExt;
use native_tls::TlsConnector;
use std::collections::HashSet;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::debug;

pub struct FetchedEmail {
    pub uid: u32,
    pub raw_bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FetchedMailboxSummary {
    pub uid: u32,
    pub is_seen: bool,
    pub size_bytes: u64,
    pub raw_header_bytes: Vec<u8>,
}

pub struct ExchangeClient;

impl ExchangeClient {
    pub async fn test_connection(cfg: &ExchangeConfig) -> Result<ExchangeTestResult, String> {
        let fut = async {
            let mut session = Self::connect(cfg).await?;
            let mailbox = session
                .select(&cfg.folder)
                .await
                .map_err(|e| format!("Failed to select folder '{}': {}", cfg.folder, e))?;

            let unseen_uids = session
                .uid_search("UNSEEN")
                .await
                .map_err(|e| format!("Failed to search unseen messages: {}", e))?;

            let _ = session.logout().await;

            Ok(ExchangeTestResult {
                message_count: mailbox.exists,
                unseen_count: unseen_uids.len() as u32,
            })
        };

        timeout(Duration::from_secs(15), fut)
            .await
            .map_err(|_| "Connection timed out after 15 seconds".to_string())?
    }

    /// Authenticate credentials against Exchange IMAP server
    pub async fn authenticate(cfg: &ExchangeConfig) -> Result<(), String> {
        let fut = async {
            let mut session = Self::connect(cfg).await?;
            let _ = session.logout().await;
            Ok(())
        };

        timeout(Duration::from_secs(15), fut)
            .await
            .map_err(|_| "Exchange authentication timed out after 15 seconds".to_string())?
    }

    pub async fn fetch_unseen_emails(
        cfg: &ExchangeConfig,
    ) -> Result<Vec<FetchedEmail>, String> {
        let fut = async {
            let mut session = Self::connect(cfg).await?;
            session
                .select(&cfg.folder)
                .await
                .map_err(|e| format!("Failed to select folder '{}': {}", cfg.folder, e))?;

            let uids: HashSet<u32> = session
                .uid_search("UNSEEN")
                .await
                .map_err(|e| format!("IMAP UNSEEN search failed: {}", e))?;

            if uids.is_empty() {
                debug!("No unseen emails found in {}", cfg.folder);
                let _ = session.logout().await;
                return Ok(Vec::new());
            }

            let mut sorted_uids: Vec<u32> = uids.into_iter().collect();
            sorted_uids.sort();

            let uid_str = sorted_uids
                .iter()
                .map(|u| u.to_string())
                .collect::<Vec<_>>()
                .join(",");

            debug!("Fetching RFC822 for UIDs: {}", uid_str);
            let mut stream = session
                .uid_fetch(&uid_str, "(RFC822 UID)")
                .await
                .map_err(|e| format!("IMAP uid_fetch failed: {}", e))?;

            let mut result = Vec::new();
            while let Some(fetch) = stream
                .try_next()
                .await
                .map_err(|e| format!("Error streaming fetch result: {}", e))?
            {
                let uid = fetch.uid.unwrap_or(0);
                if let Some(body) = fetch.body() {
                    result.push(FetchedEmail {
                        uid,
                        raw_bytes: body.to_vec(),
                    });
                }
            }

            drop(stream);
            let _ = session.logout().await;
            Ok(result)
        };

        timeout(Duration::from_secs(60), fut)
            .await
            .map_err(|_| "Fetch operation timed out after 60 seconds".to_string())?
    }

    pub async fn list_folders(cfg: &ExchangeConfig) -> Result<Vec<String>, String> {
        let fut = async {
            let mut session = Self::connect(cfg).await?;
            let mut folders = Vec::new();
            if let Ok(mut stream) = session.list(Some(""), Some("*")).await {
                while let Ok(Some(folder)) = stream.try_next().await {
                    let name = folder.name().to_string();
                    if !name.trim().is_empty() && !folders.contains(&name) {
                        folders.push(name);
                    }
                }
            }
            let _ = session.logout().await;

            if !folders.contains(&cfg.folder) {
                folders.insert(0, cfg.folder.clone());
            }
            if !folders.contains(&"INBOX".to_string()) {
                folders.push("INBOX".to_string());
            }
            Ok(folders)
        };

        timeout(Duration::from_secs(15), fut)
            .await
            .map_err(|_| "Folder list operation timed out after 15 seconds".to_string())?
    }

    pub async fn fetch_mailbox_messages(
        cfg: &ExchangeConfig,
        folder: &str,
        filter: &str,
        page: usize,
        limit: usize,
        search_query: Option<&str>,
    ) -> Result<(usize, Vec<FetchedMailboxSummary>), String> {
        let fut = async {
            let mut session = Self::connect(cfg).await?;
            let target_folder = if folder.trim().is_empty() {
                &cfg.folder
            } else {
                folder
            };
            session
                .select(target_folder)
                .await
                .map_err(|e| format!("Failed to select folder '{}': {}", target_folder, e))?;

            let uids: HashSet<u32> = if let Some(q) = search_query.map(|s| s.trim()).filter(|s| !s.is_empty()) {
                let clean_q: String = q
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '@' || *c == '.')
                    .collect();
                match session.uid_search(&format!("TEXT \"{}\"", clean_q)).await {
                    Ok(found) => found,
                    Err(_) => session
                        .uid_search("ALL")
                        .await
                        .map_err(|e| format!("IMAP search failed: {}", e))?,
                }
            } else {
                match filter.to_lowercase().as_str() {
                    "unseen" => session
                        .uid_search("UNSEEN")
                        .await
                        .map_err(|e| format!("IMAP search unseen failed: {}", e))?,
                    "seen" => session
                        .uid_search("SEEN")
                        .await
                        .map_err(|e| format!("IMAP search seen failed: {}", e))?,
                    _ => session
                        .uid_search("ALL")
                        .await
                        .map_err(|e| format!("IMAP search ALL failed: {}", e))?,
                }
            };

            if uids.is_empty() {
                let _ = session.logout().await;
                return Ok((0, Vec::new()));
            }

            let mut sorted_uids: Vec<u32> = uids.into_iter().collect();
            sorted_uids.sort_by(|a, b| b.cmp(a));

            let total = sorted_uids.len();
            let limit = limit.clamp(1, 100);
            let page = page.max(1);
            let start = (page - 1) * limit;

            if start >= total {
                let _ = session.logout().await;
                return Ok((total, Vec::new()));
            }

            let end = std::cmp::min(start + limit, total);
            let page_uids = &sorted_uids[start..end];

            let uid_str = page_uids
                .iter()
                .map(|u| u.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let mut stream = session
                .uid_fetch(&uid_str, "(BODY.PEEK[HEADER] UID FLAGS RFC822.SIZE)")
                .await
                .map_err(|e| format!("IMAP uid_fetch header failed: {}", e))?;

            let mut fetched_map = std::collections::HashMap::new();

            while let Some(fetch) = stream
                .try_next()
                .await
                .map_err(|e| format!("Error streaming fetch headers: {}", e))?
            {
                let uid = fetch.uid.unwrap_or(0);
                let is_seen = fetch.flags().any(|f| f == async_imap::types::Flag::Seen);
                let size_bytes = fetch.size.map(|s| s as u64).unwrap_or(0);
                let raw_header_bytes = fetch.header().map(|h| h.to_vec()).unwrap_or_else(|| {
                    fetch.body().map(|b| b.to_vec()).unwrap_or_default()
                });

                fetched_map.insert(
                    uid,
                    FetchedMailboxSummary {
                        uid,
                        is_seen,
                        size_bytes,
                        raw_header_bytes,
                    },
                );
            }

            drop(stream);
            let _ = session.logout().await;

            let mut results = Vec::new();
            for u in page_uids {
                if let Some(item) = fetched_map.remove(u) {
                    results.push(item);
                }
            }

            Ok((total, results))
        };

        timeout(Duration::from_secs(30), fut)
            .await
            .map_err(|_| "Fetch mailbox messages operation timed out after 30 seconds".to_string())?
    }

    pub async fn fetch_single_email_full(
        cfg: &ExchangeConfig,
        folder: &str,
        uid: u32,
    ) -> Result<(bool, Vec<u8>), String> {
        let fut = async {
            let mut session = Self::connect(cfg).await?;
            let target_folder = if folder.trim().is_empty() {
                &cfg.folder
            } else {
                folder
            };
            session
                .select(target_folder)
                .await
                .map_err(|e| format!("Failed to select folder '{}': {}", target_folder, e))?;

            let uid_str = uid.to_string();
            let mut stream = session
                .uid_fetch(&uid_str, "(BODY.PEEK[] UID FLAGS)")
                .await
                .map_err(|e| format!("IMAP uid_fetch failed for UID {}: {}", uid, e))?;

            let mut is_seen = false;
            let mut raw_bytes = Vec::new();

            if let Some(fetch) = stream
                .try_next()
                .await
                .map_err(|e| format!("Error fetching message UID {}: {}", uid, e))?
            {
                is_seen = fetch.flags().any(|f| f == async_imap::types::Flag::Seen);
                if let Some(b) = fetch.body() {
                    raw_bytes = b.to_vec();
                } else if let Some(t) = fetch.text() {
                    raw_bytes = t.to_vec();
                }
            }

            drop(stream);
            let _ = session.logout().await;

            if raw_bytes.is_empty() {
                return Err(format!("Email with UID {} was not found or has empty body", uid));
            }

            Ok((is_seen, raw_bytes))
        };

        timeout(Duration::from_secs(30), fut)
            .await
            .map_err(|_| format!("Timed out fetching full email for UID {}", uid))?
    }

    pub async fn set_read_status(
        cfg: &ExchangeConfig,
        folder: &str,
        uid: u32,
        seen: bool,
    ) -> Result<(), String> {
        let fut = async {
            let mut session = Self::connect(cfg).await?;
            let target_folder = if folder.trim().is_empty() {
                &cfg.folder
            } else {
                folder
            };
            session
                .select(target_folder)
                .await
                .map_err(|e| format!("Failed to select folder '{}': {}", target_folder, e))?;

            let uid_str = uid.to_string();
            let flag_cmd = if seen {
                "+FLAGS (\\Seen)"
            } else {
                "-FLAGS (\\Seen)"
            };

            let mut updates = session
                .uid_store(&uid_str, flag_cmd)
                .await
                .map_err(|e| format!("Failed to update flags for UID {}: {}", uid, e))?;

            while let Some(_) = updates
                .try_next()
                .await
                .map_err(|e| format!("Error updating flag: {}", e))? {}

            drop(updates);
            let _ = session.logout().await;
            Ok(())
        };

        timeout(Duration::from_secs(15), fut)
            .await
            .map_err(|_| format!("Timed out updating read status for UID {}", uid))?
    }

    pub async fn mark_as_read(cfg: &ExchangeConfig, uid: u32) -> Result<(), String> {
        Self::set_read_status(cfg, &cfg.folder, uid, true).await
    }


    async fn connect(
        cfg: &ExchangeConfig,
    ) -> Result<async_imap::Session<tokio_native_tls::TlsStream<TcpStream>>, String> {
        let addr = format!("{}:{}", cfg.host, cfg.port);
        let password = cfg
            .password
            .as_deref()
            .unwrap_or("")
            .trim();

        if password.is_empty() {
            return Err("Mailbox password is not configured".to_string());
        }

        let tcp = TcpStream::connect(&addr)
            .await
            .map_err(|e| format!("Cannot connect to {}: {}", addr, e))?;

        let mut tls_builder = TlsConnector::builder();
        if cfg.allow_self_signed {
            tls_builder.danger_accept_invalid_certs(true);
            tls_builder.danger_accept_invalid_hostnames(true);
        }

        let native_connector = tls_builder
            .build()
            .map_err(|e| format!("TLS build error: {}", e))?;
        let tokio_connector = tokio_native_tls::TlsConnector::from(native_connector);

        let tls_stream = tokio_connector
            .connect(&cfg.host, tcp)
            .await
            .map_err(|e| format!("TLS handshake failed with {}: {}", cfg.host, e))?;

        let client = async_imap::Client::new(tls_stream);
        let session = client
            .login(&cfg.username, password)
            .await
            .map_err(|(e, _)| format!("IMAP login failed for user '{}': {}", cfg.username, e))?;

        Ok(session)
    }
}
