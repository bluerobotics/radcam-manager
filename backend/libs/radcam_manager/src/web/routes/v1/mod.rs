use axum::{
    Router,
    http::StatusCode,
    routing::{get, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;

use crate::web::request_restart;

pub mod blueos;
pub mod camera;
pub mod cockpit;
pub mod info;
pub mod log;
pub mod settings;

#[instrument(level = "trace")]
pub fn router() -> Router {
    Router::new()
        .nest("/camera", camera::router())
        .nest("/log", log::router())
        .nest("/info", info::router())
        .nest("/settings", settings::router())
        .nest("/autopilot", autopilot::router())
        .route("/service/restart", post(restart))
        .route("/register_service", get(blueos::server_metadata))
        .route("/cockpit_extras.json", get(cockpit::cockpit_extras))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

#[instrument(level = "debug")]
async fn restart() -> StatusCode {
    request_restart();
    StatusCode::OK
}
