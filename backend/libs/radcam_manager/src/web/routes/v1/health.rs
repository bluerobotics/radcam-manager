use axum::{Json, Router, routing::get};
use radcam_api::SystemHealth;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;

use crate::web::connectivity;

#[instrument(level = "trace")]
pub fn router() -> Router {
    Router::new()
        .route("/", get(health))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[instrument(level = "debug")]
async fn health() -> Json<SystemHealth> {
    Json(connectivity::system_health().await)
}
