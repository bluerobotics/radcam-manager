use axum::{routing::get, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;

pub mod blueos;
pub mod camera;
pub mod cockpit;
pub mod info;
pub mod log;

#[instrument(level = "trace")]
pub fn router() -> Router {
    Router::new()
        .nest("/camera", camera::router())
        .nest("/log", log::router())
        .nest("/info", info::router())
        .route("/register_service", get(blueos::server_metadata))
        .route("/cockpit_extras.json", get(cockpit::cockpit_extras))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
