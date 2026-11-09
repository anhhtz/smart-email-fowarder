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
    assert!(html.contains("Process Monitor"));
    assert!(html.contains("tab-process"));
    assert!(html.contains("procMemRss"));
    assert!(html.contains("smtpUseExchangeCredentials"));
    assert!(html.contains("Use same credentials as Exchange Mailbox"));

    // 2. Embedded CSS
    let req_css = Request::builder().uri("/static/css/style.css").body(Body::empty()).unwrap();
    let res_css = app.clone().oneshot(req_css).await.unwrap();
    assert_eq!(res_css.status(), StatusCode::OK);
    assert_eq!(res_css.headers().get("content-type").unwrap(), "text/css; charset=utf-8");
    let css_bytes = res_css.into_body().collect().await.unwrap().to_bytes();
    let css = String::from_utf8_lossy(&css_bytes);
    assert!(css.contains("process-hero-card"));
    assert!(css.contains("credentials-badge-info"));

    // 3. Embedded JS
    let req_js = Request::builder().uri("/static/js/app.js").body(Body::empty()).unwrap();
    let res_js = app.clone().oneshot(req_js).await.unwrap();
    assert_eq!(res_js.status(), StatusCode::OK);
    assert_eq!(res_js.headers().get("content-type").unwrap(), "application/javascript; charset=utf-8");
    let js_bytes = res_js.into_body().collect().await.unwrap().to_bytes();
    let js = String::from_utf8_lossy(&js_bytes);
    assert!(js.contains("loadProcessMetrics"));
    assert!(js.contains("runHealthCheck"));
    assert!(js.contains("updateSmtpAuthUI"));
    assert!(js.contains("smtpUseExchangeCredentials"));
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
