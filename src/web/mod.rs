pub mod assets;
pub mod routes;

use crate::db::Database;
use crate::services::poller::PollerManager;
use assets::{serve_css, serve_index, serve_js};
use axum::routing::{get, post};
use axum::Router;
use routes::AppState;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn build_router(db: Database, poller: Arc<PollerManager>) -> Router {
    build_router_with_start(db, poller, chrono::Utc::now())
}

pub fn build_router_with_start(
    db: Database,
    poller: Arc<PollerManager>,
    start_time: chrono::DateTime<chrono::Utc>,
) -> Router {
    let state = AppState {
        db,
        poller,
        start_time,
    };

    let public_api_router = Router::new()
        .route("/auth/status", get(routes::get_auth_status))
        .route("/auth/providers", get(routes::get_auth_providers))
        .route("/auth/login", post(routes::auth_login))
        .route("/auth/oauth/google", get(routes::oauth_google))
        .route("/auth/oauth/google/callback", get(routes::oauth_google_callback));

    let protected_api_router = Router::new()
        .route("/auth/logout", post(routes::auth_logout))
        .route("/auth/me", get(routes::get_auth_me))
        .route("/auth/oauth/google/config", get(routes::get_google_oauth_config))
        .route("/auth/oauth/google/config", post(routes::save_google_oauth_config))
        .route("/status", get(routes::get_status))
        .route("/process", get(routes::get_process_status))
        .route("/process/health", get(routes::get_process_health))
        .route("/config", get(routes::get_config))
        .route("/config/exchange", post(routes::save_exchange_config))
        .route("/config/forwarding", post(routes::save_forwarding_config))
        .route("/config/smtp", post(routes::save_smtp_config))
        .route("/test/exchange", post(routes::test_exchange))
        .route("/test/smtp", post(routes::test_smtp))
        .route("/sync", post(routes::sync_now))
        .route("/poller/pause", post(routes::pause_poller))
        .route("/poller/resume", post(routes::resume_poller))
        .route("/logs", get(routes::get_logs))
        .route("/system-logs", get(routes::get_system_logs))
        .route("/mailbox/folders", get(routes::get_mailbox_folders))
        .route("/mailbox/messages", get(routes::get_mailbox_messages))
        .route("/mailbox/message", get(routes::get_mailbox_message))
        .route("/mailbox/attachment", get(routes::get_mailbox_attachment))
        .route("/mailbox/raw", get(routes::get_mailbox_raw))
        .route("/mailbox/forward", post(routes::forward_mailbox_message))
        .route("/mailbox/mark-read", post(routes::mark_mailbox_message_read))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            routes::auth_middleware,
        ));

    let api_router = public_api_router
        .merge(protected_api_router)
        .with_state(state);

    Router::new()
        .route("/", get(serve_index))
        .route("/index.html", get(serve_index))
        .route("/static/css/style.css", get(serve_css))
        .route("/static/js/app.js", get(serve_js))
        .nest("/api", api_router)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
