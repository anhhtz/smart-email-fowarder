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
    let state = AppState { db, poller };

    let api_router = Router::new()
        .route("/status", get(routes::get_status))
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
