use axum::{Json, Router, response::IntoResponse, routing::get};
use serde::Serialize;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;

#[derive(Serialize, Debug, Default)]
pub struct InfoContent {
    /// Name of the program
    name: String,
    /// Version/tag
    version: String,
    /// Git SHA
    sha: String,
    build_date: String,
    /// Authors name
    authors: String,
}

#[derive(Serialize, Debug, Default)]
pub struct Info {
    /// Version of the REST API
    version: u32,
    /// Service information
    service: InfoContent,
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(info))
        .route("/full", get(info_full))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[instrument(level = "trace")]
async fn info() -> Json<Info> {
    Json(Info {
        version: 0,
        service: InfoContent {
            name: option_env!("CARGO_PKG_NAME").unwrap_or("?").into(),
            version: option_env!("VERGEN_GIT_DESCRIBE").unwrap_or("?").into(),
            sha: option_env!("VERGEN_GIT_SHA").unwrap_or("?").into(),
            build_date: option_env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("?").into(),
            authors: option_env!("CARGO_PKG_AUTHORS").unwrap_or("?").into(),
        },
    })
}

#[instrument(level = "trace")]
async fn info_full() -> impl IntoResponse {
    let toml = std::str::from_utf8(include_bytes!("../../../../Cargo.toml")).unwrap();
    let content: serde_json::Value = toml::from_str(toml).unwrap();
    serde_json::to_string(&content).unwrap()
}
