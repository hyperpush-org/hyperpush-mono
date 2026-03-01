use std::sync::Arc;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use crate::state::AppState;

pub mod auth;
pub mod download;
pub mod metadata;
pub mod publish;
pub mod search;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Publish — 50MB body limit on this route only
        .route(
            "/api/v1/packages",
            post(publish::handler)
                .get(search::handler)
                .layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        // Metadata routes
        .route("/api/v1/packages/{name}", get(metadata::package_handler))
        .route("/api/v1/packages/{name}/versions", get(metadata::versions_handler))
        .route("/api/v1/packages/{name}/{version}", get(metadata::version_handler))
        // Download — streaming
        .route(
            "/api/v1/packages/{name}/{version}/download",
            get(download::handler),
        )
        // Auth routes (implemented in Plan 03)
        .route("/auth/github", get(auth::github_login))
        .route("/auth/callback", get(auth::github_callback))
        .route("/dashboard", get(auth::dashboard))
        .route("/dashboard/tokens", post(auth::create_token_handler).get(auth::list_tokens_handler))
        .with_state(state)
        .layer(CorsLayer::permissive())  // Restrict to meshlang.dev in production config
        .layer(TraceLayer::new_for_http())
}
