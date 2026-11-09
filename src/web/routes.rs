use crate::db::Database;
use crate::models::{
    ExchangeConfig, ForwardingConfig, MailboxActionRequest, MailboxAttachmentInfo,
    MailboxAttachmentQuery, MailboxFolderList, MailboxListQuery, MailboxListResponse,
    MailboxMarkReadRequest, MailboxMessageDetail, MailboxMessageQuery, MailboxMessageSummary,
    SmtpConfig, SmtpTestResult,
};
use crate::services::exchange::ExchangeClient;
use crate::services::forwarder::Forwarder;
use crate::services::poller::PollerManager;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use mail_parser::MimeHeaders;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub poller: Arc<PollerManager>,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct LogQuery {
    pub limit: Option<usize>,
}

// GET /api/status
pub async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.poller.get_status().await;
    Json(status)
}

// GET /api/process
pub async fn get_process_status(State(state): State<AppState>) -> impl IntoResponse {
    let poller_status = state.poller.get_status().await;
    let proc_status = crate::services::process::collect_process_status(
        &state.db,
        poller_status,
        state.start_time,
    );
    Json(proc_status)
}

// GET /api/process/health
pub async fn get_process_health(State(state): State<AppState>) -> impl IntoResponse {
    let report = crate::services::process::collect_health_report(&state.db, &state.poller).await;
    Json(report)
}

// GET /api/config
pub async fn get_config(State(state): State<AppState>) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let mut cfg = state
        .db
        .get_config()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    // Mask passwords for security in the UI
    if let Some(ref mut ex) = cfg.exchange {
        if ex.password.is_some() {
            ex.password = Some("••••••••".to_string());
        }
    }
    if let Some(ref mut sm) = cfg.smtp {
        if sm.password.is_some() {
            sm.password = Some("••••••••".to_string());
        }
    }

    Ok(Json(cfg))
}

// POST /api/config/exchange
pub async fn save_exchange_config(
    State(state): State<AppState>,
    Json(mut payload): Json<ExchangeConfig>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // If user typed the mask "••••••••", treat it as empty so existing is preserved
    if payload.password.as_deref() == Some("••••••••") {
        payload.password = None;
    }

    state
        .db
        .save_exchange_config(&mut payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let _ = state.db.log_event("INFO", "Exchange mailbox configuration updated.");
    Ok(Json(json!({ "success": true })))
}

// POST /api/config/forwarding
pub async fn save_forwarding_config(
    State(state): State<AppState>,
    Json(payload): Json<ForwardingConfig>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    state
        .db
        .save_forwarding_config(&payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let _ = state.db.log_event("INFO", "Forwarding rules configuration updated.");
    Ok(Json(json!({ "success": true })))
}

// POST /api/config/smtp
pub async fn save_smtp_config(
    State(state): State<AppState>,
    Json(mut payload): Json<SmtpConfig>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if payload.password.as_deref() == Some("••••••••") {
        payload.password = None;
    }
    if payload.use_exchange_credentials {
        payload.auth_required = true;
    }

    state
        .db
        .save_smtp_config(&mut payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let _ = state.db.log_event("INFO", "Exchange SMTP relay configuration updated.");
    Ok(Json(json!({ "success": true })))
}

// POST /api/test/exchange
pub async fn test_exchange(
    State(state): State<AppState>,
    Json(mut payload): Json<ExchangeConfig>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // If password was omitted or masked, read existing stored password
    if payload.password.as_deref().unwrap_or("").trim().is_empty()
        || payload.password.as_deref() == Some("••••••••")
    {
        if let Ok(cfg) = state.db.get_config() {
            if let Some(existing) = cfg.exchange {
                payload.password = existing.password;
            }
        }
    }

    match ExchangeClient::test_connection(&payload).await {
        Ok(res) => {
            let _ = state.db.log_event(
                "SUCCESS",
                &format!("Exchange connection test succeeded: {} messages, {} unseen.", res.message_count, res.unseen_count),
            );
            Ok(Json(res))
        }
        Err(e) => {
            let _ = state.db.log_event("ERROR", &format!("Exchange test failed: {}", e));
            Err((StatusCode::BAD_REQUEST, Json(json!({ "message": e }))))
        }
    }
}

// POST /api/test/smtp
pub async fn test_smtp(
    State(state): State<AppState>,
    body: Option<Json<serde_json::Value>>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state.db.get_config().unwrap_or_default();

    let effective_smtp = if let Some(Json(val)) = body {
        if let Ok(mut payload) = serde_json::from_value::<SmtpConfig>(val.clone()) {
            if !payload.use_exchange_credentials {
                if payload.password.as_deref().unwrap_or("").trim().is_empty()
                    || payload.password.as_deref() == Some("••••••••")
                {
                    if let Some(existing) = cfg.smtp.as_ref() {
                        payload.password = existing.password.clone();
                    }
                }
            }
            payload.resolve_with_exchange(cfg.exchange.as_ref())
        } else if let Some(ref ex) = cfg.exchange {
            ex.to_smtp_config()
        } else {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "message": "Exchange Mailbox is not configured. Please configure Exchange Mailbox settings first." })),
            ));
        }
    } else if let Some(ref ex) = cfg.exchange {
        ex.to_smtp_config()
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Exchange Mailbox is not configured. Please configure Exchange Mailbox settings first." })),
        ));
    };

    // Retrieve configured target Gmail address
    let target_emails = cfg
        .forwarding
        .map(|f| f.target_emails)
        .unwrap_or_else(|| vec!["admin@example.com".to_string()]);

    match Forwarder::send_test_email(&effective_smtp, &target_emails).await {
        Ok(recipient) => {
            let _ = state.db.log_event(
                "SUCCESS",
                &format!("Test email successfully sent to {}.", recipient),
            );
            Ok(Json(SmtpTestResult {
                recipient,
                message: "Test email successfully sent.".to_string(),
            }))
        }
        Err(e) => {
            let _ = state.db.log_event("ERROR", &format!("Outbound email test failed: {}", e));
            Err((StatusCode::BAD_REQUEST, Json(json!({ "message": e }))))
        }
    }
}

// POST /api/sync
pub async fn sync_now(State(state): State<AppState>) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    match state.poller.trigger_sync().await {
        Ok(res) => Ok(Json(res)),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({ "message": e })))),
    }
}

// POST /api/poller/pause
pub async fn pause_poller(State(state): State<AppState>) -> impl IntoResponse {
    state.poller.pause();
    Json(json!({ "success": true, "is_polling": false }))
}

// POST /api/poller/resume
pub async fn resume_poller(State(state): State<AppState>) -> impl IntoResponse {
    state.poller.resume();
    Json(json!({ "success": true, "is_polling": true }))
}

// GET /api/logs
pub async fn get_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let limit = query.limit.unwrap_or(50);
    let logs = state
        .db
        .get_forward_logs(limit)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    Ok(Json(logs))
}

// GET /api/system-logs
pub async fn get_system_logs(
    State(state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let limit = query.limit.unwrap_or(100);
    let logs = state
        .db
        .get_recent_system_logs(limit)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    Ok(Json(logs))
}

// Mailbox Helpers
fn format_address(addr: Option<&mail_parser::Address>) -> String {
    match addr {
        Some(mail_parser::Address::List(list)) => list
            .iter()
            .map(|a| {
                let name = a.name.as_deref().unwrap_or("").trim();
                let email = a.address.as_deref().unwrap_or("").trim();
                if name.is_empty() {
                    email.to_string()
                } else if email.is_empty() {
                    name.to_string()
                } else {
                    format!("{} <{}>", name, email)
                }
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
        Some(mail_parser::Address::Group(groups)) => groups
            .iter()
            .flat_map(|g| g.addresses.iter())
            .map(|a| {
                let name = a.name.as_deref().unwrap_or("").trim();
                let email = a.address.as_deref().unwrap_or("").trim();
                if name.is_empty() {
                    email.to_string()
                } else if email.is_empty() {
                    name.to_string()
                } else {
                    format!("{} <{}>", name, email)
                }
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
        None => String::new(),
    }
}

fn format_date(d: Option<&mail_parser::DateTime>) -> String {
    if let Some(d) = d {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            d.year, d.month, d.day, d.hour, d.minute, d.second
        )
    } else {
        "Unknown Date".to_string()
    }
}

fn format_content_type(ct: Option<&mail_parser::ContentType>) -> String {
    if let Some(ct) = ct {
        if let Some(sub) = ct.subtype() {
            format!("{}/{}", ct.c_type, sub)
        } else {
            ct.c_type.to_string()
        }
    } else {
        "application/octet-stream".to_string()
    }
}


// GET /api/mailbox/folders
pub async fn get_mailbox_folders(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state
        .db
        .get_config()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let exchange_cfg = cfg.exchange.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Exchange Mailbox is not configured. Please configure Exchange settings first." })),
        )
    })?;

    let folders = ExchangeClient::list_folders(&exchange_cfg)
        .await
        .unwrap_or_else(|_| vec![exchange_cfg.folder.clone(), "INBOX".to_string()]);

    Ok(Json(MailboxFolderList {
        current_folder: exchange_cfg.folder,
        folders,
    }))
}

// GET /api/mailbox/messages
pub async fn get_mailbox_messages(
    State(state): State<AppState>,
    Query(query): Query<MailboxListQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state
        .db
        .get_config()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let exchange_cfg = cfg.exchange.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Exchange Mailbox is not configured. Please configure Exchange settings first." })),
        )
    })?;

    let folder = query.folder.unwrap_or_else(|| exchange_cfg.folder.clone());
    let filter = query.filter.unwrap_or_else(|| "all".to_string());
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let (total, fetched) = ExchangeClient::fetch_mailbox_messages(
        &exchange_cfg,
        &folder,
        &filter,
        page,
        limit,
        query.q.as_deref(),
    )
    .await
    .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "message": e }))))?;

    let total_pages = if total == 0 { 1 } else { (total + limit - 1) / limit };

    let mut summaries = Vec::new();
    for item in fetched {
        let parsed = mail_parser::MessageParser::default().parse(&item.raw_header_bytes);

        let subject = parsed
            .as_ref()
            .and_then(|p| p.subject())
            .unwrap_or("(No Subject)")
            .to_string();

        let from = parsed
            .as_ref()
            .map(|p| format_address(p.from()))
            .unwrap_or_default();

        let to = parsed
            .as_ref()
            .map(|p| format_address(p.to()))
            .unwrap_or_default();

        let date = parsed
            .as_ref()
            .map(|p| format_date(p.date()))
            .unwrap_or_else(|| "Unknown Date".to_string());

        let message_id = parsed
            .as_ref()
            .and_then(|p| p.message_id())
            .unwrap_or("")
            .to_string();

        let is_forwarded = state.db.is_processed(&message_id, item.uid).unwrap_or(false);

        let has_attachments = parsed
            .as_ref()
            .map(|p| {
                p.content_type()
                    .map(|ct| {
                        let c = ct.c_type.as_ref().to_lowercase();
                        c.contains("multipart/mixed") || c.contains("multipart/form-data")
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        let size_formatted = crate::services::process::format_bytes(item.size_bytes);

        summaries.push(MailboxMessageSummary {
            uid: item.uid,
            message_id,
            subject,
            from,
            to,
            date,
            is_seen: item.is_seen,
            is_forwarded,
            has_attachments,
            size_formatted,
        });
    }

    Ok(Json(MailboxListResponse {
        folder,
        total,
        page,
        limit,
        total_pages,
        messages: summaries,
    }))
}

// GET /api/mailbox/message
pub async fn get_mailbox_message(
    State(state): State<AppState>,
    Query(query): Query<MailboxMessageQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state
        .db
        .get_config()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let exchange_cfg = cfg.exchange.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Exchange Mailbox is not configured. Please configure Exchange settings first." })),
        )
    })?;

    let folder = query.folder.unwrap_or_else(|| exchange_cfg.folder.clone());

    let (is_seen, raw_bytes) = ExchangeClient::fetch_single_email_full(&exchange_cfg, &folder, query.uid)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "message": e }))))?;

    let parsed = mail_parser::MessageParser::default()
        .parse(&raw_bytes)
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to parse MIME email payload" })),
            )
        })?;

    let subject = parsed.subject().unwrap_or("(No Subject)").to_string();
    let from = format_address(parsed.from());
    let to = format_address(parsed.to());
    let cc = format_address(parsed.cc());
    let date = format_date(parsed.date());
    let message_id = parsed.message_id().unwrap_or("").to_string();
    let is_forwarded = state.db.is_processed(&message_id, query.uid).unwrap_or(false);

    let body_text = parsed.body_text(0).map(|s| s.to_string()).unwrap_or_default();
    let body_html = parsed.body_html(0).map(|s| s.to_string());

    let raw_headers = {
        let bytes = &raw_bytes;
        let end_idx = bytes
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .map(|i| i + 2)
            .or_else(|| bytes.windows(2).position(|w| w == b"\n\n").map(|i| i + 1))
            .unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end_idx]).to_string()
    };

    let mut attachments = Vec::new();
    for (index, att) in parsed.attachments().enumerate() {
        let filename = att.attachment_name().unwrap_or("unnamed_attachment").to_string();
        let content_type = format_content_type(att.content_type());
        let size_bytes = att.contents().len();
        let size_formatted = crate::services::process::format_bytes(size_bytes as u64);

        attachments.push(MailboxAttachmentInfo {
            index,
            filename,
            content_type,
            size_bytes,
            size_formatted,
        });
    }

    Ok(Json(MailboxMessageDetail {
        uid: query.uid,
        message_id,
        subject,
        from,
        to,
        cc,
        date,
        is_seen,
        is_forwarded,
        body_text,
        body_html,
        raw_headers,
        attachments,
    }))
}

// GET /api/mailbox/attachment
pub async fn get_mailbox_attachment(
    State(state): State<AppState>,
    Query(query): Query<MailboxAttachmentQuery>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state
        .db
        .get_config()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let exchange_cfg = cfg.exchange.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Exchange Mailbox is not configured. Please configure Exchange settings first." })),
        )
    })?;

    let folder = query.folder.unwrap_or_else(|| exchange_cfg.folder.clone());

    let (_is_seen, raw_bytes) = ExchangeClient::fetch_single_email_full(&exchange_cfg, &folder, query.uid)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "message": e }))))?;

    let parsed = mail_parser::MessageParser::default()
        .parse(&raw_bytes)
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to parse MIME email payload" })),
            )
        })?;

    let att = parsed
        .attachments()
        .nth(query.index)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "message": format!("Attachment index {} not found", query.index) })),
            )
        })?;

    let filename = att.attachment_name().unwrap_or("attachment.bin").to_string();
    let content_type = format_content_type(att.content_type());
    let data = att.contents().to_vec();

    let safe_filename: String = filename
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();

    let res = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", safe_filename),
        )
        .body(Body::from(data))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    Ok(res)
}

// GET /api/mailbox/raw
pub async fn get_mailbox_raw(
    State(state): State<AppState>,
    Query(query): Query<MailboxMessageQuery>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state
        .db
        .get_config()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let exchange_cfg = cfg.exchange.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Exchange Mailbox is not configured. Please configure Exchange settings first." })),
        )
    })?;

    let folder = query.folder.unwrap_or_else(|| exchange_cfg.folder.clone());

    let (_is_seen, raw_bytes) = ExchangeClient::fetch_single_email_full(&exchange_cfg, &folder, query.uid)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "message": e }))))?;

    let res = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "message/rfc822")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"email-{}.eml\"", query.uid),
        )
        .body(Body::from(raw_bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    Ok(res)
}

// POST /api/mailbox/forward
pub async fn forward_mailbox_message(
    State(state): State<AppState>,
    Json(payload): Json<MailboxActionRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state
        .db
        .get_config()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let exchange_cfg = cfg.exchange.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Exchange Mailbox is not configured. Please configure Exchange settings first." })),
        )
    })?;

    let forward_cfg = cfg.forwarding.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Forwarding rules are not configured. Please configure Forwarding Rules first." })),
        )
    })?;

    let smtp_cfg = match cfg.smtp {
        Some(ref s) => s.resolve_with_exchange(Some(&exchange_cfg)),
        None => exchange_cfg.to_smtp_config(),
    };

    if forward_cfg.target_emails.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "No destination Gmail addresses configured." })),
        ));
    }

    let folder = payload.folder.unwrap_or_else(|| exchange_cfg.folder.clone());

    let (_is_seen, raw_bytes) = ExchangeClient::fetch_single_email_full(&exchange_cfg, &folder, payload.uid)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "message": e }))))?;

    let result = Forwarder::forward_email(&raw_bytes, &forward_cfg, &smtp_cfg).await;

    if result.success {
        let _ = state.db.mark_processed(&result.message_id, payload.uid, &result.subject);
        let _ = state.db.record_forward_log(
            &result.message_id,
            &result.original_sender,
            &result.subject,
            &result.forwarded_to,
            "SUCCESS",
            None,
        );
        let _ = state.db.log_event(
            "SUCCESS",
            &format!(
                "Manual Forward: '{}' from {} -> {}",
                result.subject, result.original_sender, result.forwarded_to
            ),
        );

        if exchange_cfg.mark_as_read {
            let _ = ExchangeClient::set_read_status(&exchange_cfg, &folder, payload.uid, true).await;
        }

        Ok(Json(json!({
            "success": true,
            "message": format!("Successfully forwarded to {}", result.forwarded_to),
            "forwarded_to": result.forwarded_to,
            "subject": result.subject
        })))
    } else {
        let err_text = result.error.as_deref().unwrap_or("Unknown error");
        let _ = state.db.record_forward_log(
            &result.message_id,
            &result.original_sender,
            &result.subject,
            &result.forwarded_to,
            "FAILED",
            Some(err_text),
        );
        let _ = state.db.log_event(
            "ERROR",
            &format!("Manual forward failed for '{}': {}", result.subject, err_text),
        );

        Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": format!("Forward failed: {}", err_text) })),
        ))
    }
}

// POST /api/mailbox/mark-read
pub async fn mark_mailbox_message_read(
    State(state): State<AppState>,
    Json(payload): Json<MailboxMarkReadRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let cfg = state
        .db
        .get_config()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let exchange_cfg = cfg.exchange.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Exchange Mailbox is not configured. Please configure Exchange settings first." })),
        )
    })?;

    let folder = payload.folder.unwrap_or_else(|| exchange_cfg.folder.clone());

    ExchangeClient::set_read_status(&exchange_cfg, &folder, payload.uid, payload.seen)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "message": e }))))?;

    let _ = state.db.log_event(
        "INFO",
        &format!(
            "Marked email UID {} as {} in folder '{}'",
            payload.uid,
            if payload.seen { "read" } else { "unread" },
            folder
        ),
    );

    Ok(Json(json!({ "success": true, "uid": payload.uid, "is_seen": payload.seen })))
}

