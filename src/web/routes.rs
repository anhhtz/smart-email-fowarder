use crate::db::Database;
use crate::models::{
    AuthProviderInfo, AuthProvidersResponse, AuthStatusResponse, ExchangeConfig,
    ForwardingConfig, GoogleOAuthConfig, LoginRequest, MailboxActionRequest,
    MailboxAttachmentInfo, MailboxAttachmentQuery, MailboxFolderList, MailboxListQuery,
    MailboxListResponse, MailboxMarkReadRequest, MailboxMessageDetail, MailboxMessageQuery,
    MailboxMessageSummary, SmtpConfig, SmtpTestResult, UserSession,
};
use crate::services::exchange::ExchangeClient;
use crate::services::forwarder::Forwarder;
use crate::services::google_oauth::GoogleOAuthService;
use crate::services::poller::PollerManager;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
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

#[derive(Debug, Deserialize)]
pub struct OAuthInitQuery {
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub fn extract_token(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(auth_val) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(t) = auth_val.strip_prefix("Bearer ") {
            return Some(t.trim().to_string());
        }
    }
    if let Some(tok_val) = headers.get("X-Auth-Token").and_then(|v| v.to_str().ok()) {
        return Some(tok_val.trim().to_string());
    }
    if let Some(cookie_hdr) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        for part in cookie_hdr.split(';') {
            let part = part.trim();
            if let Some(t) = part.strip_prefix("auth_token=") {
                return Some(t.trim().to_string());
            }
        }
    }
    None
}

// Authentication Middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let token = extract_token(req.headers());

    if let Some(tok) = token {
        match state.db.validate_auth_session(&tok) {
            Ok(Some(session)) => {
                if session.provider == "google" && !GoogleOAuthService::is_email_allowed(&session.username) {
                    let _ = state.db.delete_auth_session(&tok);
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({
                            "error": "Forbidden",
                            "message": format!("Access revoked: '{}' is not in the ALLOWED_EMAILS list.", session.username)
                        })),
                    ).into_response();
                }
                req.extensions_mut().insert(session);
                return next.run(req).await;
            }
            Ok(None) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "Unauthorized",
                        "message": "Session has expired or is invalid. Please sign in again with Exchange mailbox credentials."
                    })),
                ).into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Database Error",
                        "message": e.to_string()
                    })),
                ).into_response();
            }
        }
    }

    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": "Unauthorized",
            "message": "Authentication required. Please sign in with your Exchange mailbox credentials."
        })),
    ).into_response()
}

pub fn usernames_match(input: &str, configured: &str, configured_email: Option<&str>) -> bool {
    let input_clean = input.trim().to_lowercase();
    let conf_clean = configured.trim().to_lowercase();

    if input_clean == conf_clean {
        return true;
    }

    if let Some(email) = configured_email {
        if input_clean == email.trim().to_lowercase() {
            return true;
        }
    }

    let input_base = input_clean
        .split('\\')
        .last()
        .unwrap_or(&input_clean)
        .split('@')
        .next()
        .unwrap_or(&input_clean);

    let conf_base = conf_clean
        .split('\\')
        .last()
        .unwrap_or(&conf_clean)
        .split('@')
        .next()
        .unwrap_or(&conf_clean);

    !input_base.is_empty() && input_base == conf_base
}

// GET /api/auth/status
pub async fn get_auth_status(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let cfg = state.db.get_config().ok();
    let exchange_cfg = cfg.as_ref().and_then(|c| c.exchange.as_ref());
    let exchange_configured = exchange_cfg.is_some();
    let default_host = exchange_cfg.map(|e| e.host.clone());
    let default_username = exchange_cfg.map(|e| e.username.clone());
    let default_allow_self_signed = exchange_cfg.map(|e| e.allow_self_signed).unwrap_or(false);
    let default_port = exchange_cfg.map(|e| e.port);
    let default_security = exchange_cfg.map(|e| e.security.clone());

    let db_oauth = state.db.get_google_oauth_config().ok();
    let google_cfg = GoogleOAuthService::get_config(db_oauth.as_ref());
    let google_configured = GoogleOAuthService::is_configured(&google_cfg);

    let token = extract_token(&headers);
    let user = if let Some(t) = token {
        match state.db.validate_auth_session(&t).unwrap_or(None) {
            Some(session) => {
                if session.provider == "google" && !GoogleOAuthService::is_email_allowed(&session.username) {
                    let _ = state.db.delete_auth_session(&t);
                    None
                } else {
                    Some(session)
                }
            }
            None => None,
        }
    } else {
        None
    };

    let default_provider = Some(if google_configured {
        "google".to_string()
    } else {
        "exchange".to_string()
    });

    let authenticated = user.is_some();
    Json(AuthStatusResponse {
        authenticated,
        user,
        exchange_configured,
        google_configured,
        default_provider,
        default_host,
        default_username,
        default_allow_self_signed,
        default_port,
        default_security,
    })
}

// GET /api/auth/providers
pub async fn get_auth_providers(State(state): State<AppState>) -> impl IntoResponse {
    let exchange_configured = state
        .db
        .get_config()
        .ok()
        .and_then(|c| c.exchange)
        .is_some();

    let db_oauth = state.db.get_google_oauth_config().ok();
    let google_cfg = GoogleOAuthService::get_config(db_oauth.as_ref());
    let google_configured = GoogleOAuthService::is_configured(&google_cfg);

    let providers = vec![
        AuthProviderInfo {
            id: "google".to_string(),
            name: "Google Workspace / Gmail OAuth2".to_string(),
            description: if google_configured {
                "Authenticate securely using your Google Workspace or personal Gmail account.".to_string()
            } else {
                "Google OAuth2 is waiting for credentials in .env (GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET).".to_string()
            },
            is_active: google_configured,
            stage: if google_configured { "active".to_string() } else { "needs_config".to_string() },
            icon: "google".to_string(),
        },
        AuthProviderInfo {
            id: "exchange".to_string(),
            name: "Microsoft Exchange Mailbox".to_string(),
            description: "Authenticate directly with Exchange IMAP and mailbox credentials.".to_string(),
            is_active: true,
            stage: "active".to_string(),
            icon: "exchange".to_string(),
        },
    ];

    Json(AuthProvidersResponse {
        providers,
        exchange_configured,
    })
}

// POST /api/auth/login
pub async fn auth_login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let provider = payload.provider.as_deref().unwrap_or("exchange");

    if provider == "google" {
        return Err((
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "success": false,
                "message": "Password authentication is not applicable for Google OAuth2. Please click 'Sign In with Google' to sign in."
            })),
        ));
    }

    if provider != "exchange" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": format!("Unsupported authentication provider: '{}'", provider)
            })),
        ));
    }

    let username = payload.username.trim();
    let password = payload.password.trim();

    if username.is_empty() || password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Username and password are required."
            })),
        ));
    }

    // Resolve Exchange configuration to authenticate against
    let existing_cfg = state.db.get_config().ok().and_then(|c| c.exchange);

    let host = payload
        .host
        .as_deref()
        .filter(|h| !h.trim().is_empty())
        .or_else(|| existing_cfg.as_ref().map(|c| c.host.as_str()));

    let host = match host {
        Some(h) => h.to_string(),
        None => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": "Exchange server host is not configured yet. Please provide the server hostname (e.g. mail.company.com)."
                })),
            ));
        }
    };

    let is_same_host = existing_cfg
        .as_ref()
        .map_or(false, |ex| host.eq_ignore_ascii_case(&ex.host));

    let port = if is_same_host {
        payload.port.unwrap_or_else(|| existing_cfg.as_ref().unwrap().port)
    } else {
        payload.port.unwrap_or(993)
    };

    let security = if is_same_host {
        payload
            .security
            .clone()
            .unwrap_or_else(|| existing_cfg.as_ref().unwrap().security.clone())
    } else {
        payload.security.unwrap_or_else(|| "ssl".to_string())
    };

    let allow_self_signed = if is_same_host {
        let ex_self_signed = existing_cfg.as_ref().unwrap().allow_self_signed;
        ex_self_signed || payload.allow_self_signed.unwrap_or(false)
    } else {
        payload.allow_self_signed.unwrap_or(false)
    };

    // Check if input username matches existing configured Exchange mailbox user
    let is_configured_user = existing_cfg.as_ref().map_or(false, |old| {
        usernames_match(username, &old.username, old.from_email.as_deref())
    });

    let effective_username = if is_configured_user {
        existing_cfg.as_ref().unwrap().username.clone()
    } else {
        username.to_string()
    };

    let password_matches_stored = existing_cfg.as_ref().map_or(false, |old| {
        is_configured_user && old.password.as_deref() == Some(password)
    });

    let mut target_cfg = ExchangeConfig {
        host: host.clone(),
        port,
        security,
        username: effective_username.clone(),
        password: Some(password.to_string()),
        folder: existing_cfg
            .as_ref()
            .map(|c| c.folder.clone())
            .unwrap_or_else(|| "INBOX".to_string()),
        mark_as_read: existing_cfg
            .as_ref()
            .map(|c| c.mark_as_read)
            .unwrap_or(true),
        allow_self_signed,
        smtp_port: existing_cfg
            .as_ref()
            .map(|c| c.smtp_port)
            .unwrap_or(587),
        smtp_security: existing_cfg
            .as_ref()
            .map(|c| c.smtp_security.clone())
            .unwrap_or_else(|| "starttls".to_string()),
        from_email: existing_cfg
            .as_ref()
            .and_then(|c| c.from_email.clone()),
        from_name: existing_cfg
            .as_ref()
            .map(|c| c.from_name.clone())
            .unwrap_or_else(|| "Exchange Email Forwarder".to_string()),
    };

    // Try live authentication with Exchange
    let mut live_auth_result = ExchangeClient::authenticate(&target_cfg).await;

    // If effective username (e.g. CORP\user) failed or was different from typed username, also try typed username
    if live_auth_result.is_err() && effective_username != username {
        let mut alt_cfg = target_cfg.clone();
        alt_cfg.username = username.to_string();
        if let Ok(()) = ExchangeClient::authenticate(&alt_cfg).await {
            target_cfg.username = username.to_string();
            live_auth_result = Ok(());
        }
    }

    match live_auth_result {
        Ok(()) => {
            // Live verification succeeded
            if existing_cfg.is_none() || is_configured_user {
                let _ = state.db.save_exchange_config(&mut target_cfg);
            }
        }
        Err(err) => {
            if password_matches_stored {
                let _ = state.db.log_event(
                    "WARN",
                    &format!(
                        "Exchange server live authentication returned ({}), authenticated user '{}' via configured mailbox credentials.",
                        err, username
                    ),
                );
            } else {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "success": false,
                        "message": format!("Exchange authentication failed: {}", err)
                    })),
                ));
            }
        }
    }

    // Create session in database
    let session_user = if is_configured_user {
        existing_cfg.as_ref().unwrap().username.clone()
    } else {
        username.to_string()
    };

    let session = state
        .db
        .create_auth_session(&session_user, "exchange", 24)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": e.to_string() })),
            )
        })?;

    let _ = state.db.log_event(
        "SUCCESS",
        &format!("User '{}' authenticated via Exchange mailbox.", session_user),
    );

    let cookie_val = format!(
        "auth_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        session.token,
        24 * 3600
    );

    let mut headers = axum::http::HeaderMap::new();
    if let Ok(val) = cookie_val.parse() {
        headers.insert(header::SET_COOKIE, val);
    }

    Ok((
        headers,
        Json(json!({
            "success": true,
            "token": session.token,
            "user": session
        })),
    ))
}

// POST /api/auth/logout
pub async fn auth_logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    if let Some(token) = extract_token(&headers) {
        let _ = state.db.delete_auth_session(&token);
    }

    let cookie_val = "auth_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
    let mut resp_headers = axum::http::HeaderMap::new();
    if let Ok(val) = cookie_val.parse() {
        resp_headers.insert(header::SET_COOKIE, val);
    }

    (
        resp_headers,
        Json(json!({
            "success": true,
            "message": "Signed out successfully."
        })),
    )
}

// GET /api/auth/me
pub async fn get_auth_me(
    axum::extract::Extension(session): axum::extract::Extension<UserSession>,
) -> impl IntoResponse {
    Json(session)
}

fn generate_random_token() -> String {
    let mut bytes = [0u8; 16];
    for b in bytes.iter_mut() {
        *b = fastrand::u8(..);
    }
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// GET /api/auth/oauth/google
pub async fn oauth_google(
    State(state): State<AppState>,
    Query(query): Query<OAuthInitQuery>,
) -> impl IntoResponse {
    let db_oauth = state.db.get_google_oauth_config().ok();
    let google_cfg = GoogleOAuthService::get_config(db_oauth.as_ref());
    let is_configured = GoogleOAuthService::is_configured(&google_cfg);

    if !is_configured {
        let err_body = json!({
            "status": "unconfigured",
            "provider": "google",
            "is_configured": false,
            "message": "Google OAuth2 is not configured. Please define GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET in your .env file.",
            "env_parameters": {
                "GOOGLE_CLIENT_ID": "OAuth 2.0 Client ID from Google Cloud Console",
                "GOOGLE_CLIENT_SECRET": "OAuth 2.0 Client Secret",
                "GOOGLE_REDIRECT_URI": google_cfg.redirect_uri
            }
        });

        if query.format.as_deref() == Some("json") {
            return (StatusCode::BAD_REQUEST, Json(err_body)).into_response();
        } else {
            return (
                StatusCode::BAD_REQUEST,
                Html(format!(
                    r#"<!DOCTYPE html>
                    <html lang="en">
                    <head>
                        <meta charset="utf-8">
                        <title>Google OAuth2 Not Configured</title>
                        <style>
                            body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f172a; color: #f8fafc; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; padding: 20px; box-sizing: border-box; }}
                            .card {{ background: #1e293b; padding: 32px; border-radius: 12px; box-shadow: 0 10px 25px rgba(0,0,0,0.5); max-width: 560px; border: 1px solid #334155; }}
                            h2 {{ color: #f59e0b; margin-top: 0; }}
                            p {{ color: #94a3b8; font-size: 14px; line-height: 1.6; }}
                            code {{ background: #0f172a; color: #38bdf8; padding: 2px 6px; border-radius: 4px; font-size: 13px; }}
                            pre {{ background: #0f172a; color: #38bdf8; padding: 12px; border-radius: 6px; font-size: 12px; overflow-x: auto; }}
                            .btn {{ display: inline-block; margin-top: 16px; padding: 10px 20px; background: #2563eb; color: white; text-decoration: none; border-radius: 6px; font-weight: 500; font-size: 14px; }}
                            .btn:hover {{ background: #1d4ed8; }}
                        </style>
                    </head>
                    <body>
                        <div class="card">
                            <h2>Google OAuth2 Credentials Required</h2>
                            <p>To enable Google OAuth2 authentication, provide your credentials in the <code>.env</code> file:</p>
                            <pre>GOOGLE_CLIENT_ID=your_client_id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your_client_secret
GOOGLE_REDIRECT_URI={}</pre>
                            <p>Once set, restart the server and sign in with Google.</p>
                            <a href="/" class="btn">Return to Login</a>
                        </div>
                    </body>
                    </html>"#,
                    html_escape(&google_cfg.redirect_uri)
                )),
            ).into_response();
        }
    }

    // Generate CSRF state token
    let csrf_state = generate_random_token();

    match GoogleOAuthService::build_authorization_url(&google_cfg, &csrf_state) {
        Ok(auth_url) => {
            if query.format.as_deref() == Some("json") {
                Json(json!({
                    "status": "ready",
                    "provider": "google",
                    "is_configured": true,
                    "auth_url": auth_url,
                    "redirect_uri": google_cfg.redirect_uri,
                    "scopes": google_cfg.scopes
                })).into_response()
            } else {
                Redirect::temporary(&auth_url).into_response()
            }
        }
        Err(err) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": format!("Failed to build Google authorization URL: {}", err)
                })),
            ).into_response()
        }
    }
}

// GET /api/auth/oauth/google/callback
pub async fn oauth_google_callback(
    State(state): State<AppState>,
    Query(query): Query<OAuthCallbackQuery>,
) -> impl IntoResponse {
    // 1. Handle error returned by Google
    if let Some(err) = query.error {
        let desc = query.error_description.unwrap_or(err);
        return Html(format!(
            r#"<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="utf-8">
                <title>Google Sign-In Failed</title>
                <style>
                    body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f172a; color: #f8fafc; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
                    .card {{ background: #1e293b; padding: 32px; border-radius: 12px; box-shadow: 0 10px 25px rgba(0,0,0,0.5); max-width: 480px; text-align: center; border: 1px solid #334155; }}
                    h2 {{ color: #ef4444; margin-top: 0; }}
                    p {{ color: #94a3b8; font-size: 14px; line-height: 1.5; }}
                    .btn {{ display: inline-block; margin-top: 20px; padding: 10px 20px; background: #2563eb; color: white; text-decoration: none; border-radius: 6px; font-weight: 500; font-size: 14px; }}
                </style>
            </head>
            <body>
                <div class="card">
                    <h2>Google Sign-In Failed</h2>
                    <p>{}</p>
                    <a href="/" class="btn">Return to Login</a>
                </div>
            </body>
            </html>"#,
            html_escape(&desc)
        )).into_response();
    }

    // 2. Validate authorization code
    let code = match query.code {
        Some(c) if !c.trim().is_empty() => c,
        _ => {
            return Html(r#"<!DOCTYPE html>
            <html lang="en">
            <head><meta charset="utf-8"><title>Error</title></head>
            <body style="font-family: sans-serif; background: #0f172a; color: #f8fafc; display: flex; align-items: center; justify-content: center; height: 100vh;">
                <div style="background: #1e293b; padding: 30px; border-radius: 8px; text-align: center;">
                    <h2 style="color: #ef4444;">Missing Authorization Code</h2>
                    <p style="color: #94a3b8;">No authorization code received from Google.</p>
                    <a href="/" style="color: #38bdf8;">Return to Login</a>
                </div>
            </body>
            </html>"#.to_string()).into_response();
        }
    };

    let db_oauth = state.db.get_google_oauth_config().ok();
    let google_cfg = GoogleOAuthService::get_config(db_oauth.as_ref());

    if !GoogleOAuthService::is_configured(&google_cfg) {
        return Html(r#"<!DOCTYPE html>
        <html lang="en">
        <head><meta charset="utf-8"><title>Error</title></head>
        <body style="font-family: sans-serif; background: #0f172a; color: #f8fafc; display: flex; align-items: center; justify-content: center; height: 100vh;">
            <div style="background: #1e293b; padding: 30px; border-radius: 8px; text-align: center;">
                <h2 style="color: #ef4444;">OAuth Configuration Incomplete</h2>
                <p style="color: #94a3b8;">Google OAuth credentials are not properly configured in .env.</p>
                <a href="/" style="color: #38bdf8;">Return to Login</a>
            </div>
        </body>
        </html>"#.to_string()).into_response();
    }

    // 3. Exchange code for user info
    match GoogleOAuthService::exchange_code_for_user_info(&google_cfg, &code).await {
        Ok(user_info) => {
            // Check whitelist if ALLOWED_EMAILS is configured
            if !GoogleOAuthService::is_email_allowed(&user_info.email) {
                let _ = state.db.log_event(
                    "WARN",
                    &format!(
                        "Rejected Google OAuth login attempt for email '{}': not in ALLOWED_EMAILS whitelist.",
                        user_info.email
                    ),
                );

                return Html(format!(
                    r#"<!DOCTYPE html>
                    <html lang="en">
                    <head>
                        <meta charset="utf-8">
                        <title>Access Denied</title>
                        <style>
                            body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f172a; color: #f8fafc; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
                            .card {{ background: #1e293b; padding: 36px; border-radius: 12px; box-shadow: 0 10px 25px rgba(0,0,0,0.5); max-width: 500px; text-align: center; border: 1px solid #334155; }}
                            h2 {{ color: #ef4444; margin-top: 0; }}
                            p {{ color: #94a3b8; font-size: 14px; line-height: 1.6; }}
                            code {{ background: #0f172a; color: #f87171; padding: 2px 6px; border-radius: 4px; font-size: 13px; font-weight: 600; }}
                            .btn {{ display: inline-block; margin-top: 20px; padding: 10px 20px; background: #2563eb; color: white; text-decoration: none; border-radius: 6px; font-weight: 500; font-size: 14px; }}
                            .btn:hover {{ background: #1d4ed8; }}
                        </style>
                    </head>
                    <body>
                        <div class="card">
                            <h2>Access Denied (403)</h2>
                            <p>The Google account <code>{}</code> is not authorized to sign in to this application.</p>
                            <p>Only email addresses specified in <code>ALLOWED_EMAILS</code> are granted access.</p>
                            <a href="/" class="btn">Return to Login</a>
                        </div>
                    </body>
                    </html>"#,
                    html_escape(&user_info.email)
                )).into_response();
            }

            let session = match state.db.create_auth_session(&user_info.email, "google", 24) {
                Ok(s) => s,
                Err(e) => {
                    return Html(format!(
                        r#"<!DOCTYPE html><html><body style="background:#0f172a;color:#f8fafc;padding:40px;"><h2>Failed to create session: {}</h2><a href="/" style="color:#38bdf8;">Return to Login</a></body></html>"#,
                        html_escape(&e.to_string())
                    )).into_response();
                }
            };

            let _ = state.db.log_event(
                "SUCCESS",
                &format!("User '{}' authenticated via Google OAuth2.", user_info.email),
            );

            let cookie_val = format!(
                "auth_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
                session.token,
                24 * 3600
            );

            let mut resp_headers = axum::http::HeaderMap::new();
            if let Ok(val) = cookie_val.parse() {
                resp_headers.insert(header::SET_COOKIE, val);
            }

            let display_name = user_info.name.unwrap_or_else(|| user_info.email.clone());
            let safe_token = session.token.replace('\'', "\\'");
            let safe_email = user_info.email.replace('\'', "\\'");
            let safe_name = display_name.replace('\'', "\\'");

            let html_body = format!(
                r#"<!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta charset="utf-8">
                    <title>Signing in with Google...</title>
                    <style>
                        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f172a; color: #f8fafc; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
                        .card {{ text-align: center; background: #1e293b; padding: 36px 48px; border-radius: 12px; border: 1px solid #334155; box-shadow: 0 10px 25px rgba(0,0,0,0.5); }}
                        .spinner {{ border: 3px solid rgba(255,255,255,0.1); border-top: 3px solid #38bdf8; border-radius: 50%; width: 36px; height: 36px; animation: spin 1s linear infinite; margin: 0 auto 16px; }}
                        @keyframes spin {{ 0% {{ transform: rotate(0deg); }} 100% {{ transform: rotate(360deg); }} }}
                        p {{ color: #94a3b8; font-size: 14px; margin: 0; }}
                    </style>
                </head>
                <body>
                    <div class="card">
                        <div class="spinner"></div>
                        <h3 style="margin: 0 0 8px 0; color: #38bdf8;">Authentication Successful</h3>
                        <p>Redirecting to dashboard...</p>
                    </div>
                    <script>
                        try {{
                            localStorage.setItem('auth_token', '{safe_token}');
                            localStorage.setItem('auth_user', JSON.stringify({{
                                username: '{safe_email}',
                                name: '{safe_name}',
                                provider: 'google'
                            }}));
                        }} catch (e) {{
                            console.error("Storage error:", e);
                        }}
                        window.location.replace('/');
                    </script>
                </body>
                </html>"#,
                safe_token = safe_token,
                safe_email = safe_email,
                safe_name = safe_name
            );

            (resp_headers, Html(html_body)).into_response()
        }
        Err(err) => {
            Html(format!(
                r#"<!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta charset="utf-8">
                    <title>Google Sign-In Failed</title>
                    <style>
                        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f172a; color: #f8fafc; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
                        .card {{ background: #1e293b; padding: 32px; border-radius: 12px; box-shadow: 0 10px 25px rgba(0,0,0,0.5); max-width: 480px; text-align: center; border: 1px solid #334155; }}
                        h2 {{ color: #ef4444; margin-top: 0; }}
                        p {{ color: #94a3b8; font-size: 14px; line-height: 1.5; }}
                        .btn {{ display: inline-block; margin-top: 20px; padding: 10px 20px; background: #2563eb; color: white; text-decoration: none; border-radius: 6px; font-weight: 500; font-size: 14px; }}
                        .btn:hover {{ background: #1d4ed8; }}
                    </style>
                </head>
                <body>
                    <div class="card">
                        <h2>Google Authentication Failed</h2>
                        <p>{}</p>
                        <a href="/" class="btn">Return to Login</a>
                    </div>
                </body>
                </html>"#,
                html_escape(&err)
            )).into_response()
        }
    }
}

// GET /api/auth/oauth/google/config
pub async fn get_google_oauth_config(State(state): State<AppState>) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let db_cfg = state.db.get_google_oauth_config().ok();
    let effective = GoogleOAuthService::get_config(db_cfg.as_ref());
    let is_configured = GoogleOAuthService::is_configured(&effective);

    let has_env_client_id = std::env::var("GOOGLE_CLIENT_ID")
        .map(|v| !v.trim().is_empty() && v != "your_google_client_id_here")
        .unwrap_or(false);
    let has_env_client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
        .map(|v| !v.trim().is_empty() && v != "your_google_client_secret_here")
        .unwrap_or(false);

    let mut response_cfg = effective.clone();
    if response_cfg.client_secret.is_some() {
        response_cfg.client_secret = Some("••••••••".to_string());
    }

    Ok(Json(json!({
        "client_id": response_cfg.client_id,
        "client_secret": response_cfg.client_secret,
        "redirect_uri": response_cfg.redirect_uri,
        "scopes": response_cfg.scopes,
        "enabled": is_configured,
        "is_configured": is_configured,
        "from_env": has_env_client_id && has_env_client_secret,
        "env_var_names": {
            "client_id": "GOOGLE_CLIENT_ID",
            "client_secret": "GOOGLE_CLIENT_SECRET",
            "redirect_uri": "GOOGLE_REDIRECT_URI"
        }
    })))
}

// POST /api/auth/oauth/google/config (Slot Config)
pub async fn save_google_oauth_config(
    State(state): State<AppState>,
    Json(mut payload): Json<GoogleOAuthConfig>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    if payload.client_secret.as_deref() == Some("••••••••") {
        if let Ok(existing) = state.db.get_google_oauth_config() {
            payload.client_secret = existing.client_secret;
        }
    }

    state.db.save_google_oauth_config(&payload)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "message": e.to_string() }))))?;

    let _ = state.db.log_event("INFO", "Google OAuth2 slot configuration updated.");
    Ok(Json(json!({ "success": true, "message": "Google OAuth2 slot configuration saved." })))
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

