use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use smart_email_forwarder::db::Database;
use smart_email_forwarder::services::poller::PollerManager;
use smart_email_forwarder::web;
use std::sync::Arc;
use tower::util::ServiceExt;

fn create_test_app() -> axum::Router {
    let db = Database::new_in_memory().unwrap();
    let (poller, _rx) = PollerManager::new(db.clone());
    web::build_router(db, Arc::new(poller))
}

#[tokio::test]
async fn test_serve_embedded_frontend() {
    let app = create_test_app();

    // 1. Root HTML
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&body_bytes);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("Smart Email Forwarder"));
    assert!(html.contains("Email Browser"));
    assert!(html.contains("data-tab=\"mailbox\""));
    assert!(html.contains("tab-mailbox"));
    assert!(html.contains("mailboxFolderSelect"));
    assert!(html.contains("mailboxReaderView"));
    assert!(html.contains("Process Monitor"));
    assert!(html.contains("tab-process"));
    assert!(html.contains("procMemRss"));
    assert!(html.contains("tab-exchange"));
    assert!(html.contains("tab-forwarding"));
    assert!(!html.contains("data-tab=\"smtp\""));
    assert!(!html.contains("id=\"tab-smtp\""));

    // 2. Embedded CSS
    let req_css = Request::builder().uri("/static/css/style.css").body(Body::empty()).unwrap();
    let res_css = app.clone().oneshot(req_css).await.unwrap();
    assert_eq!(res_css.status(), StatusCode::OK);
    assert_eq!(res_css.headers().get("content-type").unwrap(), "text/css; charset=utf-8");
    let css_bytes = res_css.into_body().collect().await.unwrap().to_bytes();
    let css = String::from_utf8_lossy(&css_bytes);
    assert!(css.contains("process-hero-card"));
    assert!(css.contains("mailbox-toolbar"));
    assert!(css.contains("email-reader-card"));

    // 3. Embedded JS
    let req_js = Request::builder().uri("/static/js/app.js").body(Body::empty()).unwrap();
    let res_js = app.clone().oneshot(req_js).await.unwrap();
    assert_eq!(res_js.status(), StatusCode::OK);
    assert_eq!(res_js.headers().get("content-type").unwrap(), "application/javascript; charset=utf-8");
    let js_bytes = res_js.into_body().collect().await.unwrap().to_bytes();
    let js = String::from_utf8_lossy(&js_bytes);
    assert!(js.contains("loadProcessMetrics"));
    assert!(js.contains("runHealthCheck"));
    assert!(js.contains("sendTestEmail"));
    assert!(js.contains("loadMailboxMessages"));
    assert!(js.contains("openMailboxMessage"));
}

#[tokio::test]
async fn test_api_status_and_config() {
    let app = create_test_app();

    // 1. GET /api/status
    let req = Request::builder().uri("/api/status").body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["is_polling"], true);

    // 2. GET /api/config
    let req_cfg = Request::builder().uri("/api/config").body(Body::empty()).unwrap();
    let res_cfg = app.clone().oneshot(req_cfg).await.unwrap();
    assert_eq!(res_cfg.status(), StatusCode::OK);

    // 3. POST /api/config/smtp with use_exchange_credentials
    let smtp_payload = serde_json::json!({
        "host": "mail.company.local",
        "port": 25,
        "security": "plain",
        "from_email": "forwarder@company.com",
        "from_name": "Relay",
        "auth_required": false,
        "use_exchange_credentials": true,
        "username": "",
        "password": null
    });
    let req_save_smtp = Request::builder()
        .method("POST")
        .uri("/api/config/smtp")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&smtp_payload).unwrap()))
        .unwrap();
    let res_save_smtp = app.clone().oneshot(req_save_smtp).await.unwrap();
    assert_eq!(res_save_smtp.status(), StatusCode::OK);

    // Verify GET /api/config has use_exchange_credentials and auth_required = true
    let req_cfg2 = Request::builder().uri("/api/config").body(Body::empty()).unwrap();
    let res_cfg2 = app.clone().oneshot(req_cfg2).await.unwrap();
    let bytes2 = res_cfg2.into_body().collect().await.unwrap().to_bytes();
    let val2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(val2["smtp"]["use_exchange_credentials"], true);
    assert_eq!(val2["smtp"]["auth_required"], true);
}

#[tokio::test]
async fn test_api_poller_pause_resume() {
    let app = create_test_app();

    // 1. Pause
    let req_pause = Request::builder()
        .method("POST")
        .uri("/api/poller/pause")
        .body(Body::empty())
        .unwrap();
    let res_pause = app.clone().oneshot(req_pause).await.unwrap();
    assert_eq!(res_pause.status(), StatusCode::OK);

    // Check status
    let req_status = Request::builder().uri("/api/status").body(Body::empty()).unwrap();
    let res_status = app.clone().oneshot(req_status).await.unwrap();
    let bytes = res_status.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["is_polling"], false);

    // 2. Resume
    let req_resume = Request::builder()
        .method("POST")
        .uri("/api/poller/resume")
        .body(Body::empty())
        .unwrap();
    let res_resume = app.clone().oneshot(req_resume).await.unwrap();
    assert_eq!(res_resume.status(), StatusCode::OK);

    // Check status again
    let req_status2 = Request::builder().uri("/api/status").body(Body::empty()).unwrap();
    let res_status2 = app.clone().oneshot(req_status2).await.unwrap();
    let bytes2 = res_status2.into_body().collect().await.unwrap().to_bytes();
    let val2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(val2["is_polling"], true);
}

#[tokio::test]
async fn test_api_process_and_health() {
    let app = create_test_app();

    // 1. GET /api/process
    let req_proc = Request::builder().uri("/api/process").body(Body::empty()).unwrap();
    let res_proc = app.clone().oneshot(req_proc).await.unwrap();
    assert_eq!(res_proc.status(), StatusCode::OK);
    let bytes = res_proc.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(val["pid"].as_u64().unwrap() > 0);
    assert_eq!(val["status"], "Running");
    assert!(val["uptime_formatted"].as_str().is_some());
    assert!(val["database"]["is_healthy"].as_bool().unwrap());
    assert_eq!(val["poller"]["is_polling"], true);
    assert!(val["memory"]["rss_formatted"].as_str().is_some());

    // 2. GET /api/process/health
    let req_health = Request::builder().uri("/api/process/health").body(Body::empty()).unwrap();
    let res_health = app.clone().oneshot(req_health).await.unwrap();
    assert_eq!(res_health.status(), StatusCode::OK);
    let bytes_health = res_health.into_body().collect().await.unwrap().to_bytes();
    let val_health: serde_json::Value = serde_json::from_slice(&bytes_health).unwrap();
    assert!(val_health["overall_status"].as_str().is_some());
    assert!(val_health["checks"].as_array().unwrap().len() >= 4);
}

#[tokio::test]
async fn test_api_test_smtp_with_exchange_credentials() {
    let app = create_test_app();

    // Testing SMTP with use_exchange_credentials when Exchange config is not yet set
    // should fail gracefully with a descriptive error message
    let test_payload = serde_json::json!({
        "host": "127.0.0.1",
        "port": 25,
        "security": "plain",
        "from_email": "test@company.local",
        "from_name": "Test",
        "auth_required": true,
        "use_exchange_credentials": true,
        "username": "",
        "password": null
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/test/smtp")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&test_payload).unwrap()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let msg = val["message"].as_str().unwrap();
    assert!(msg.contains("Exchange mailbox credentials"));
}

#[tokio::test]
async fn test_api_test_smtp_default_exchange_mailbox() {
    let app = create_test_app();

    // 1. First save Exchange mailbox settings
    let ex_payload = serde_json::json!({
        "host": "127.0.0.1",
        "port": 993,
        "security": "ssl",
        "username": "user@company.local",
        "password": "Password123!",
        "folder": "INBOX",
        "mark_as_read": true,
        "allow_self_signed": true,
        "smtp_port": 2525,
        "smtp_security": "plain",
        "from_email": "user@company.local",
        "from_name": "Test User"
    });
    let req_save_ex = Request::builder()
        .method("POST")
        .uri("/api/config/exchange")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&ex_payload).unwrap()))
        .unwrap();
    let res_save_ex = app.clone().oneshot(req_save_ex).await.unwrap();
    assert_eq!(res_save_ex.status(), StatusCode::OK);

    // 2. Call /api/test/smtp with empty body {} - it should use the Exchange mailbox by default
    let req_test = Request::builder()
        .method("POST")
        .uri("/api/test/smtp")
        .header("content-type", "application/json")
        .body(Body::from(b"{}" as &[u8]))
        .unwrap();
    let res_test = app.clone().oneshot(req_test).await.unwrap();
    // Since 127.0.0.1:2525 is not running a real SMTP server in this test, it will return BAD_REQUEST
    // but the error message will be a connection error, NOT missing exchange config!
    assert_eq!(res_test.status(), StatusCode::BAD_REQUEST);
    let bytes = res_test.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let msg = val["message"].as_str().unwrap();
    // It attempted connection using the Exchange config host/port!
    assert!(!msg.contains("Exchange Mailbox is not configured"));
}

#[tokio::test]
async fn test_api_mailbox_unconfigured() {
    let app = create_test_app();

    // 1. GET /api/mailbox/folders without config
    let req_folders = Request::builder().uri("/api/mailbox/folders").body(Body::empty()).unwrap();
    let res_folders = app.clone().oneshot(req_folders).await.unwrap();
    assert_eq!(res_folders.status(), StatusCode::BAD_REQUEST);
    let bytes = res_folders.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(val["message"].as_str().unwrap().contains("Exchange Mailbox is not configured"));

    // 2. GET /api/mailbox/messages without config
    let req_msgs = Request::builder().uri("/api/mailbox/messages").body(Body::empty()).unwrap();
    let res_msgs = app.clone().oneshot(req_msgs).await.unwrap();
    assert_eq!(res_msgs.status(), StatusCode::BAD_REQUEST);

    // 3. GET /api/mailbox/message without config
    let req_msg = Request::builder().uri("/api/mailbox/message?uid=123").body(Body::empty()).unwrap();
    let res_msg = app.clone().oneshot(req_msg).await.unwrap();
    assert_eq!(res_msg.status(), StatusCode::BAD_REQUEST);

    // 4. POST /api/mailbox/forward without config
    let fwd_payload = serde_json::json!({ "uid": 123 });
    let req_fwd = Request::builder()
        .method("POST")
        .uri("/api/mailbox/forward")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&fwd_payload).unwrap()))
        .unwrap();
    let res_fwd = app.clone().oneshot(req_fwd).await.unwrap();
    assert_eq!(res_fwd.status(), StatusCode::BAD_REQUEST);

    // 5. POST /api/mailbox/mark-read without config
    let read_payload = serde_json::json!({ "uid": 123, "seen": true });
    let req_read = Request::builder()
        .method("POST")
        .uri("/api/mailbox/mark-read")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&read_payload).unwrap()))
        .unwrap();
    let res_read = app.clone().oneshot(req_read).await.unwrap();
    assert_eq!(res_read.status(), StatusCode::BAD_REQUEST);
}

