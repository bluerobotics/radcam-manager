use axum::{
    Json, Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use radcam_commands::CameraControl;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;

use crate::web::control_bridge;

#[instrument(level = "trace")]
pub fn router() -> Router {
    Router::new()
        .route("/list", get(radcam_commands::list))
        .route("/control", post(control))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

#[instrument(level = "debug")]
async fn control(Json(camera_control): Json<CameraControl>) -> impl IntoResponse {
    match control_bridge::camera_control(camera_control).await {
        Ok(value) => (StatusCode::OK, value.to_string()).into_response(),
        Err(error) => {
            let status =
                StatusCode::from_u16(error.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, error.message).into_response()
        }
    }
}
