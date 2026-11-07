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

    pub async fn mark_as_read(cfg: &ExchangeConfig, uid: u32) -> Result<(), String> {
        let fut = async {
            let mut session = Self::connect(cfg).await?;
            session
                .select(&cfg.folder)
                .await
                .map_err(|e| format!("Failed to select folder '{}': {}", cfg.folder, e))?;

            let uid_str = uid.to_string();
            let mut updates = session
                .uid_store(&uid_str, "+FLAGS (\\Seen)")
                .await
                .map_err(|e| format!("Failed to set \\Seen flag for UID {}: {}", uid, e))?;

            while let Some(_) = updates
                .try_next()
                .await
                .map_err(|e| format!("Error updating \\Seen flag: {}", e))? {}

            drop(updates);
            let _ = session.logout().await;
            Ok(())
        };

        timeout(Duration::from_secs(15), fut)
            .await
            .map_err(|_| format!("Timed out marking UID {} as read", uid))?
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
