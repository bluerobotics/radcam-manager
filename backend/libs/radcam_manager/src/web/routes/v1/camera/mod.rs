use axum::{
    Router,
    routing::{get, post},
};
use radcam_commands;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;

#[instrument(level = "trace")]
pub fn router() -> Router {
    Router::new()
        .route("/list", get(radcam_commands::list))
        .route("/control", post(radcam_commands::control))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
