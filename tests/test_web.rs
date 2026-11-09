use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use smart_email_forwarder::db::Database;
use smart_email_forwarder::services::poller::PollerManager;
use smart_email_forwarder::web;
use std::sync::Arc;
use tower::util::ServiceExt;

struct TestContext {
    app: axum::Router,
    db: Database,
    token: String,
}

fn create_test_context() -> TestContext {
    std::env::set_var("SMART_EMAIL_TEST", "1");
    let db = Database::new_in_memory().unwrap();
    let session = db
        .create_auth_session("admin@company.local", "exchange", 24)
        .unwrap();
    let (poller, _rx) = PollerManager::new(db.clone());
    let app = web::build_router(db.clone(), Arc::new(poller));
    TestContext {
        app,
        db,
        token: session.token,
    }
}

fn authed_req(method: &str, uri: &str, token: &str) -> axum::http::request::Builder {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {}", token))
}

#[tokio::test]
async fn test_serve_embedded_frontend() {
    let ctx = create_test_context();
    let app = ctx.app;

    // 1. Root HTML (Public)
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
    assert!(html.contains("authScreen"));
    assert!(html.contains("Exchange Mailbox"));
    assert!(html.contains("Google OAuth2"));
    assert!(html.contains("authProviderTag"));

    // 2. Embedded CSS (Public)
    let req_css = Request::builder().uri("/static/css/style.css").body(Body::empty()).unwrap();
    let res_css = app.clone().oneshot(req_css).await.unwrap();
    assert_eq!(res_css.status(), StatusCode::OK);
    assert_eq!(res_css.headers().get("content-type").unwrap(), "text/css; charset=utf-8");
    let css_bytes = res_css.into_body().collect().await.unwrap().to_bytes();
    let css = String::from_utf8_lossy(&css_bytes);
    assert!(css.contains("auth-overlay"));
    assert!(css.contains("auth-card"));
    assert!(css.contains("process-hero-card"));

    // 3. Embedded JS (Public)
    let req_js = Request::builder().uri("/static/js/app.js").body(Body::empty()).unwrap();
    let res_js = app.clone().oneshot(req_js).await.unwrap();
    assert_eq!(res_js.status(), StatusCode::OK);
    assert_eq!(res_js.headers().get("content-type").unwrap(), "application/javascript; charset=utf-8");
    let js_bytes = res_js.into_body().collect().await.unwrap().to_bytes();
    let js = String::from_utf8_lossy(&js_bytes);
    assert!(js.contains("checkAuthStatus"));
    assert!(js.contains("authScreen"));
    assert!(js.contains("btnSubmitExchangeLogin"));
}

#[tokio::test]
async fn test_unauthenticated_requests_fail_with_401() {
    let ctx = create_test_context();
    let app = ctx.app;

    // Protected endpoints without auth token must return 401 Unauthorized
    let endpoints = vec![
        ("/api/status", "GET"),
        ("/api/config", "GET"),
        ("/api/process", "GET"),
        ("/api/process/health", "GET"),
        ("/api/mailbox/folders", "GET"),
        ("/api/logs", "GET"),
        ("/api/poller/pause", "POST"),
    ];

    for (uri, method) in endpoints {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::UNAUTHORIZED,
            "Endpoint {} should require authentication",
            uri
        );
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(val["error"], "Unauthorized");
    }
}

#[tokio::test]
async fn test_auth_status_and_providers() {
    let ctx = create_test_context();
    let app = ctx.app;

    // 1. Unauthenticated status check (public)
    let req_unauth = Request::builder().uri("/api/auth/status").body(Body::empty()).unwrap();
    let res_unauth = app.clone().oneshot(req_unauth).await.unwrap();
    assert_eq!(res_unauth.status(), StatusCode::OK);
    let bytes_unauth = res_unauth.into_body().collect().await.unwrap().to_bytes();
    let val_unauth: serde_json::Value = serde_json::from_slice(&bytes_unauth).unwrap();
    assert_eq!(val_unauth["authenticated"], false);
    assert!(val_unauth["user"].is_null());

    // 2. Authenticated status check with Bearer token
    let req_auth = authed_req("GET", "/api/auth/status", &ctx.token)
        .body(Body::empty())
        .unwrap();
    let res_auth = app.clone().oneshot(req_auth).await.unwrap();
    assert_eq!(res_auth.status(), StatusCode::OK);
    let bytes_auth = res_auth.into_body().collect().await.unwrap().to_bytes();
    let val_auth: serde_json::Value = serde_json::from_slice(&bytes_auth).unwrap();
    assert_eq!(val_auth["authenticated"], true);
    assert_eq!(val_auth["user"]["username"], "admin@company.local");
    assert_eq!(val_auth["user"]["provider"], "exchange");

    // 3. Auth Providers (public) - must list Exchange (active) and Google OAuth2 (slot for next stage)
    let req_prov = Request::builder().uri("/api/auth/providers").body(Body::empty()).unwrap();
    let res_prov = app.clone().oneshot(req_prov).await.unwrap();
    assert_eq!(res_prov.status(), StatusCode::OK);
    let bytes_prov = res_prov.into_body().collect().await.unwrap().to_bytes();
    let val_prov: serde_json::Value = serde_json::from_slice(&bytes_prov).unwrap();
    let providers = val_prov["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 2);
    assert_eq!(providers[0]["id"], "google");
    assert_eq!(providers[1]["id"], "exchange");

    let google_prov = &providers[0];
    assert_eq!(google_prov["is_active"], false);
    assert_eq!(google_prov["stage"], "needs_config");

    let exchange_prov = &providers[1];
    assert_eq!(exchange_prov["is_active"], true);
    assert_eq!(exchange_prov["stage"], "active");
}

#[tokio::test]
async fn test_auth_exchange_login_and_logout() {
    let ctx = create_test_context();
    let app = ctx.app;

    // 1. Save an Exchange configuration in the DB
    let mut ex_cfg = smart_email_forwarder::models::ExchangeConfig {
        host: "mail.company.local".to_string(),
        port: 993,
        security: "ssl".to_string(),
        username: "jdoe@company.local".to_string(),
        password: Some("SecretPassword123!".to_string()),
        folder: "INBOX".to_string(),
        mark_as_read: true,
        allow_self_signed: true,
        smtp_port: 587,
        smtp_security: "starttls".to_string(),
        from_email: None,
        from_name: "John Doe".to_string(),
    };
    ctx.db.save_exchange_config(&mut ex_cfg).unwrap();

    // 2. Attempt login with wrong password
    let bad_login = serde_json::json!({
        "username": "jdoe@company.local",
        "password": "WrongPassword"
    });
    let req_bad = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&bad_login).unwrap()))
        .unwrap();
    let res_bad = app.clone().oneshot(req_bad).await.unwrap();
    assert_eq!(res_bad.status(), StatusCode::UNAUTHORIZED);

    // 3. Attempt login with correct Exchange credentials
    let good_login = serde_json::json!({
        "username": "jdoe@company.local",
        "password": "SecretPassword123!"
    });
    let req_good = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&good_login).unwrap()))
        .unwrap();
    let res_good = app.clone().oneshot(req_good).await.unwrap();
    assert_eq!(res_good.status(), StatusCode::OK);

    // Verify Set-Cookie header contains auth_token
    let cookie = res_good.headers().get("set-cookie").unwrap().to_str().unwrap();
    assert!(cookie.contains("auth_token="));

    let bytes_good = res_good.into_body().collect().await.unwrap().to_bytes();
    let val_good: serde_json::Value = serde_json::from_slice(&bytes_good).unwrap();
    assert_eq!(val_good["success"], true);
    let new_token = val_good["token"].as_str().unwrap();
    assert_eq!(val_good["user"]["username"], "jdoe@company.local");
    assert_eq!(val_good["user"]["provider"], "exchange");

    // Also verify username variation login (e.g. typing "jdoe" without @company.local or domain prefix)
    let good_login_alt = serde_json::json!({
        "username": "jdoe",
        "password": "SecretPassword123!"
    });
    let req_good_alt = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&good_login_alt).unwrap()))
        .unwrap();
    let res_good_alt = app.clone().oneshot(req_good_alt).await.unwrap();
    assert_eq!(res_good_alt.status(), StatusCode::OK);

    // 4. Verify /api/auth/me works with the newly acquired token
    let req_me = authed_req("GET", "/api/auth/me", new_token)
        .body(Body::empty())
        .unwrap();
    let res_me = app.clone().oneshot(req_me).await.unwrap();
    assert_eq!(res_me.status(), StatusCode::OK);
    let bytes_me = res_me.into_body().collect().await.unwrap().to_bytes();
    let val_me: serde_json::Value = serde_json::from_slice(&bytes_me).unwrap();
    assert_eq!(val_me["username"], "jdoe@company.local");

    // 5. Logout with the token
    let req_logout = authed_req("POST", "/api/auth/logout", new_token)
        .body(Body::empty())
        .unwrap();
    let res_logout = app.clone().oneshot(req_logout).await.unwrap();
    assert_eq!(res_logout.status(), StatusCode::OK);

    // 6. Accessing /api/auth/me with the invalidated token must now return 401
    let req_me_expired = authed_req("GET", "/api/auth/me", new_token)
        .body(Body::empty())
        .unwrap();
    let res_me_expired = app.clone().oneshot(req_me_expired).await.unwrap();
    assert_eq!(res_me_expired.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_google_oauth2_endpoints() {
    let ctx = create_test_context();
    let app = ctx.app;

    // 1. Direct password login with provider: "google" should return 501 Not Implemented
    let google_login = serde_json::json!({
        "provider": "google",
        "username": "user@gmail.com",
        "password": "any"
    });
    let req_g_login = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&google_login).unwrap()))
        .unwrap();
    let res_g_login = app.clone().oneshot(req_g_login).await.unwrap();
    assert_eq!(res_g_login.status(), StatusCode::NOT_IMPLEMENTED);

    // 2. GET /api/auth/oauth/google?format=json when unconfigured returns 400 with helpful config guidance
    let req_unconf_json = Request::builder()
        .uri("/api/auth/oauth/google?format=json")
        .body(Body::empty())
        .unwrap();
    let res_unconf_json = app.clone().oneshot(req_unconf_json).await.unwrap();
    assert_eq!(res_unconf_json.status(), StatusCode::BAD_REQUEST);
    let bytes_unconf = res_unconf_json.into_body().collect().await.unwrap().to_bytes();
    let val_unconf: serde_json::Value = serde_json::from_slice(&bytes_unconf).unwrap();
    assert_eq!(val_unconf["status"], "unconfigured");
    assert_eq!(val_unconf["is_configured"], false);
    assert!(val_unconf["message"].as_str().unwrap().contains(".env"));

    // 3. GET /api/auth/oauth/google (HTML) when unconfigured returns 400 Bad Request HTML page
    let req_unconf_html = Request::builder()
        .uri("/api/auth/oauth/google")
        .body(Body::empty())
        .unwrap();
    let res_unconf_html = app.clone().oneshot(req_unconf_html).await.unwrap();
    assert_eq!(res_unconf_html.status(), StatusCode::BAD_REQUEST);
    let bytes_unconf_html = res_unconf_html.into_body().collect().await.unwrap().to_bytes();
    let html_unconf = String::from_utf8_lossy(&bytes_unconf_html);
    assert!(html_unconf.contains("Google OAuth2 Credentials Required"));
    assert!(html_unconf.contains("GOOGLE_CLIENT_ID"));

    // 4. Configure Google OAuth2 credentials
    let save_payload = serde_json::json!({
        "enabled": true,
        "client_id": "test-client-id.apps.googleusercontent.com",
        "client_secret": "test-secret-value",
        "redirect_uri": "http://localhost:8080/api/auth/oauth/google/callback",
        "scopes": ["openid", "email", "profile", "https://mail.google.com/"]
    });
    let req_save = authed_req("POST", "/api/auth/oauth/google/config", &ctx.token)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&save_payload).unwrap()))
        .unwrap();
    let res_save = app.clone().oneshot(req_save).await.unwrap();
    assert_eq!(res_save.status(), StatusCode::OK);

    // 5. GET /api/auth/oauth/google?format=json when configured returns 200 and authorization URL
    let req_conf_json = Request::builder()
        .uri("/api/auth/oauth/google?format=json")
        .body(Body::empty())
        .unwrap();
    let res_conf_json = app.clone().oneshot(req_conf_json).await.unwrap();
    assert_eq!(res_conf_json.status(), StatusCode::OK);
    let bytes_conf = res_conf_json.into_body().collect().await.unwrap().to_bytes();
    let val_conf: serde_json::Value = serde_json::from_slice(&bytes_conf).unwrap();
    assert_eq!(val_conf["status"], "ready");
    assert_eq!(val_conf["is_configured"], true);
    let auth_url = val_conf["auth_url"].as_str().unwrap();
    assert!(auth_url.starts_with("https://accounts.google.com/o/oauth2/v2/auth"));
    assert!(auth_url.contains("client_id=test-client-id.apps.googleusercontent.com"));

    // 6. GET /api/auth/oauth/google (browser navigation) redirects via 307 Temporary Redirect
    let req_conf_nav = Request::builder()
        .uri("/api/auth/oauth/google")
        .body(Body::empty())
        .unwrap();
    let res_conf_nav = app.clone().oneshot(req_conf_nav).await.unwrap();
    assert_eq!(res_conf_nav.status(), StatusCode::TEMPORARY_REDIRECT);
    let loc = res_conf_nav.headers().get("location").unwrap().to_str().unwrap();
    assert!(loc.starts_with("https://accounts.google.com/o/oauth2/v2/auth"));

    // 7. GET /api/auth/oauth/google/callback with Google error param returns error HTML
    let req_err_cb = Request::builder()
        .uri("/api/auth/oauth/google/callback?error=access_denied&error_description=User+cancelled+request")
        .body(Body::empty())
        .unwrap();
    let res_err_cb = app.clone().oneshot(req_err_cb).await.unwrap();
    assert_eq!(res_err_cb.status(), StatusCode::OK);
    let bytes_err_cb = res_err_cb.into_body().collect().await.unwrap().to_bytes();
    let html_err_cb = String::from_utf8_lossy(&bytes_err_cb);
    assert!(html_err_cb.contains("Google Sign-In Failed"));
    assert!(html_err_cb.contains("User cancelled request"));

    // 8. GET /api/auth/oauth/google/callback with missing code returns missing code message
    let req_no_code = Request::builder()
        .uri("/api/auth/oauth/google/callback")
        .body(Body::empty())
        .unwrap();
    let res_no_code = app.clone().oneshot(req_no_code).await.unwrap();
    assert_eq!(res_no_code.status(), StatusCode::OK);
    let bytes_no_code = res_no_code.into_body().collect().await.unwrap().to_bytes();
    let html_no_code = String::from_utf8_lossy(&bytes_no_code);
    assert!(html_no_code.contains("Missing Authorization Code"));
}

#[tokio::test]
async fn test_google_oauth2_configuration_persistence() {
    let ctx = create_test_context();
    let app = ctx.app;

    // 1. Initial GET /api/auth/oauth/google/config
    let req_cfg = authed_req("GET", "/api/auth/oauth/google/config", &ctx.token)
        .body(Body::empty())
        .unwrap();
    let res_cfg = app.clone().oneshot(req_cfg).await.unwrap();
    assert_eq!(res_cfg.status(), StatusCode::OK);
    let bytes_cfg = res_cfg.into_body().collect().await.unwrap().to_bytes();
    let val_cfg: serde_json::Value = serde_json::from_slice(&bytes_cfg).unwrap();
    assert_eq!(val_cfg["redirect_uri"], "http://localhost:8080/api/auth/oauth/google/callback");

    // 2. POST /api/auth/oauth/google/config
    let save_payload = serde_json::json!({
        "enabled": true,
        "client_id": "123456789.apps.googleusercontent.com",
        "client_secret": "GOCSPX-SecretValue123",
        "redirect_uri": "http://localhost:8080/api/auth/oauth/google/callback",
        "scopes": ["openid", "email", "profile", "https://mail.google.com/"]
    });
    let req_save = authed_req("POST", "/api/auth/oauth/google/config", &ctx.token)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&save_payload).unwrap()))
        .unwrap();
    let res_save = app.clone().oneshot(req_save).await.unwrap();
    assert_eq!(res_save.status(), StatusCode::OK);

    // 3. Verify client_id is preserved and secret is masked
    let req_check = authed_req("GET", "/api/auth/oauth/google/config", &ctx.token)
        .body(Body::empty())
        .unwrap();
    let res_check = app.clone().oneshot(req_check).await.unwrap();
    assert_eq!(res_check.status(), StatusCode::OK);
    let bytes_check = res_check.into_body().collect().await.unwrap().to_bytes();
    let val_check: serde_json::Value = serde_json::from_slice(&bytes_check).unwrap();
    assert_eq!(val_check["client_id"], "123456789.apps.googleusercontent.com");
    assert_eq!(val_check["client_secret"], "••••••••");
    assert_eq!(val_check["is_configured"], true);
}

#[tokio::test]
async fn test_api_status_and_config() {
    let ctx = create_test_context();
    let app = ctx.app;

    // 1. GET /api/status
    let req = authed_req("GET", "/api/status", &ctx.token).body(Body::empty()).unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["is_polling"], true);

    // 2. GET /api/config
    let req_cfg = authed_req("GET", "/api/config", &ctx.token).body(Body::empty()).unwrap();
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
    let req_save_smtp = authed_req("POST", "/api/config/smtp", &ctx.token)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&smtp_payload).unwrap()))
        .unwrap();
    let res_save_smtp = app.clone().oneshot(req_save_smtp).await.unwrap();
    assert_eq!(res_save_smtp.status(), StatusCode::OK);

    // Verify GET /api/config has use_exchange_credentials and auth_required = true
    let req_cfg2 = authed_req("GET", "/api/config", &ctx.token).body(Body::empty()).unwrap();
    let res_cfg2 = app.clone().oneshot(req_cfg2).await.unwrap();
    let bytes2 = res_cfg2.into_body().collect().await.unwrap().to_bytes();
    let val2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(val2["smtp"]["use_exchange_credentials"], true);
    assert_eq!(val2["smtp"]["auth_required"], true);
}

#[tokio::test]
async fn test_api_poller_pause_resume() {
    let ctx = create_test_context();
    let app = ctx.app;

    // 1. Pause
    let req_pause = authed_req("POST", "/api/poller/pause", &ctx.token)
        .body(Body::empty())
        .unwrap();
    let res_pause = app.clone().oneshot(req_pause).await.unwrap();
    assert_eq!(res_pause.status(), StatusCode::OK);

    // Check status
    let req_status = authed_req("GET", "/api/status", &ctx.token).body(Body::empty()).unwrap();
    let res_status = app.clone().oneshot(req_status).await.unwrap();
    let bytes = res_status.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["is_polling"], false);

    // 2. Resume
    let req_resume = authed_req("POST", "/api/poller/resume", &ctx.token)
        .body(Body::empty())
        .unwrap();
    let res_resume = app.clone().oneshot(req_resume).await.unwrap();
    assert_eq!(res_resume.status(), StatusCode::OK);

    // Check status again
    let req_status2 = authed_req("GET", "/api/status", &ctx.token).body(Body::empty()).unwrap();
    let res_status2 = app.clone().oneshot(req_status2).await.unwrap();
    let bytes2 = res_status2.into_body().collect().await.unwrap().to_bytes();
    let val2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(val2["is_polling"], true);
}

#[tokio::test]
async fn test_api_process_and_health() {
    let ctx = create_test_context();
    let app = ctx.app;

    // 1. GET /api/process
    let req_proc = authed_req("GET", "/api/process", &ctx.token).body(Body::empty()).unwrap();
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
    let req_health = authed_req("GET", "/api/process/health", &ctx.token).body(Body::empty()).unwrap();
    let res_health = app.clone().oneshot(req_health).await.unwrap();
    assert_eq!(res_health.status(), StatusCode::OK);
    let bytes_health = res_health.into_body().collect().await.unwrap().to_bytes();
    let val_health: serde_json::Value = serde_json::from_slice(&bytes_health).unwrap();
    assert!(val_health["overall_status"].as_str().is_some());
    assert!(val_health["checks"].as_array().unwrap().len() >= 4);
}

#[tokio::test]
async fn test_api_test_smtp_with_exchange_credentials() {
    let ctx = create_test_context();
    let app = ctx.app;

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
    let req = authed_req("POST", "/api/test/smtp", &ctx.token)
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
    let ctx = create_test_context();
    let app = ctx.app;

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
    let req_save_ex = authed_req("POST", "/api/config/exchange", &ctx.token)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&ex_payload).unwrap()))
        .unwrap();
    let res_save_ex = app.clone().oneshot(req_save_ex).await.unwrap();
    assert_eq!(res_save_ex.status(), StatusCode::OK);

    // 2. Call /api/test/smtp with empty body {}
    let req_test = authed_req("POST", "/api/test/smtp", &ctx.token)
        .header("content-type", "application/json")
        .body(Body::from(b"{}" as &[u8]))
        .unwrap();
    let res_test = app.clone().oneshot(req_test).await.unwrap();
    assert_eq!(res_test.status(), StatusCode::BAD_REQUEST);
    let bytes = res_test.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let msg = val["message"].as_str().unwrap();
    assert!(!msg.contains("Exchange Mailbox is not configured"));
}

#[tokio::test]
async fn test_api_mailbox_unconfigured() {
    let ctx = create_test_context();
    let app = ctx.app;

    // 1. GET /api/mailbox/folders without config
    let req_folders = authed_req("GET", "/api/mailbox/folders", &ctx.token).body(Body::empty()).unwrap();
    let res_folders = app.clone().oneshot(req_folders).await.unwrap();
    assert_eq!(res_folders.status(), StatusCode::BAD_REQUEST);
    let bytes = res_folders.into_body().collect().await.unwrap().to_bytes();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(val["message"].as_str().unwrap().contains("Exchange Mailbox is not configured"));

    // 2. GET /api/mailbox/messages without config
    let req_msgs = authed_req("GET", "/api/mailbox/messages", &ctx.token).body(Body::empty()).unwrap();
    let res_msgs = app.clone().oneshot(req_msgs).await.unwrap();
    assert_eq!(res_msgs.status(), StatusCode::BAD_REQUEST);

    // 3. GET /api/mailbox/message without config
    let req_msg = authed_req("GET", "/api/mailbox/message?uid=123", &ctx.token).body(Body::empty()).unwrap();
    let res_msg = app.clone().oneshot(req_msg).await.unwrap();
    assert_eq!(res_msg.status(), StatusCode::BAD_REQUEST);

    // 4. POST /api/mailbox/forward without config
    let fwd_payload = serde_json::json!({ "uid": 123 });
    let req_fwd = authed_req("POST", "/api/mailbox/forward", &ctx.token)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&fwd_payload).unwrap()))
        .unwrap();
    let res_fwd = app.clone().oneshot(req_fwd).await.unwrap();
    assert_eq!(res_fwd.status(), StatusCode::BAD_REQUEST);

    // 5. POST /api/mailbox/mark-read without config
    let read_payload = serde_json::json!({ "uid": 123, "seen": true });
    let req_read = authed_req("POST", "/api/mailbox/mark-read", &ctx.token)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&read_payload).unwrap()))
        .unwrap();
    let res_read = app.clone().oneshot(req_read).await.unwrap();
    assert_eq!(res_read.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn test_google_oauth2_email_whitelist() {
    std::env::set_var("SMART_EMAIL_TEST", "1");
    use smart_email_forwarder::services::google_oauth::GoogleOAuthService;

    // 1. Unset or empty ALLOWED_EMAILS allows any email
    std::env::remove_var("ALLOWED_EMAILS");
    assert!(GoogleOAuthService::is_email_allowed("anyone@gmail.com"));

    // 2. Set ALLOWED_EMAILS with multiple emails, spaces, and Gmail addresses with dots
    std::env::set_var("ALLOWED_EMAILS", "admin@company.com, CEO@agribank.com.vn ; staff@corp.vn, anhht.ca@gmail.com");
    assert!(GoogleOAuthService::is_email_allowed("admin@company.com"));
    assert!(GoogleOAuthService::is_email_allowed("ceo@agribank.com.vn")); // case-insensitive
    assert!(GoogleOAuthService::is_email_allowed("STAFF@corp.vn"));

    // Gmail normalization tests:
    assert!(GoogleOAuthService::is_email_allowed("anhhtca@gmail.com")); // stripped dot matches anhht.ca
    assert!(GoogleOAuthService::is_email_allowed("Anhht.Ca@gmail.com")); // case + dot
    assert!(GoogleOAuthService::is_email_allowed("anhht.ca+notifications@gmail.com")); // plus tag
    assert!(GoogleOAuthService::is_email_allowed("anhhtca@googlemail.com")); // googlemail domain

    // Disallowed emails
    assert!(!GoogleOAuthService::is_email_allowed("stranger@gmail.com"));
    assert!(!GoogleOAuthService::is_email_allowed("attacker@hacker.io"));

    // Clean up
    std::env::remove_var("ALLOWED_EMAILS");
    std::env::remove_var("SMART_EMAIL_TEST");
}
