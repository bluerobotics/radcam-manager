use axum::{Router, http::StatusCode, routing::delete};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;

#[instrument(level = "trace")]
pub fn router() -> Router {
    Router::new()
        .route("/", delete(clear))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

#[instrument(level = "debug")]
async fn clear() -> StatusCode {
    match autopilot::clear_saved_settings().await {
        Ok(()) => StatusCode::OK,
        Err(error) => {
            warn!("Failed to clear settings: {error:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
