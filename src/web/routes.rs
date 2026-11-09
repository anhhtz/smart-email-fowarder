use crate::db::Database;
use crate::models::{
    ExchangeConfig, ForwardingConfig, SmtpConfig, SmtpTestResult,
};
use crate::services::exchange::ExchangeClient;
use crate::services::forwarder::Forwarder;
use crate::services::poller::PollerManager;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
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
    Json(mut payload): Json<SmtpConfig>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if payload.password.as_deref().unwrap_or("").trim().is_empty()
        || payload.password.as_deref() == Some("••••••••")
    {
        if let Ok(cfg) = state.db.get_config() {
            if let Some(existing) = cfg.smtp {
                payload.password = existing.password;
            }
        }
    }

    // Retrieve configured target Gmail address
    let cfg = state.db.get_config().unwrap_or_default();
    let target_emails = cfg
        .forwarding
        .map(|f| f.target_emails)
        .unwrap_or_else(|| vec!["admin@example.com".to_string()]);

    match Forwarder::send_test_email(&payload, &target_emails).await {
        Ok(recipient) => {
            let _ = state.db.log_event(
                "SUCCESS",
                &format!("Test email successfully relayed to {}.", recipient),
            );
            Ok(Json(SmtpTestResult {
                recipient,
                message: "Test email successfully relayed.".to_string(),
            }))
        }
        Err(e) => {
            let _ = state.db.log_event("ERROR", &format!("SMTP relay test failed: {}", e));
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
