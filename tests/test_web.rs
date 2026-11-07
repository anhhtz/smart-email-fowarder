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

    // 2. Embedded CSS
    let req_css = Request::builder().uri("/static/css/style.css").body(Body::empty()).unwrap();
    let res_css = app.clone().oneshot(req_css).await.unwrap();
    assert_eq!(res_css.status(), StatusCode::OK);
    assert_eq!(res_css.headers().get("content-type").unwrap(), "text/css; charset=utf-8");

    // 3. Embedded JS
    let req_js = Request::builder().uri("/static/js/app.js").body(Body::empty()).unwrap();
    let res_js = app.clone().oneshot(req_js).await.unwrap();
    assert_eq!(res_js.status(), StatusCode::OK);
    assert_eq!(res_js.headers().get("content-type").unwrap(), "application/javascript; charset=utf-8");
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
