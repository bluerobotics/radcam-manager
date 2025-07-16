use axum::{Json, Router, response::IntoResponse, routing::get};
use reqwest::StatusCode;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;

use crate::ActuatorsConfig;

pub fn router() -> Router {
    Router::new()
        .route("/", get(get_config).post(set_config).delete(reset_config))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[instrument(level = "trace")]
async fn get_config() -> Json<ActuatorsConfig> {
    Json(crate::get_config().await.into())
}

#[instrument(level = "trace")]
async fn set_config(new_config: Json<ActuatorsConfig>) -> impl IntoResponse {
    let res = match crate::set_config(&new_config).await {
        Ok(res) => res,
        Err(error) => {
            warn!("res from set_config: {error:#?}");
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error:?}")).into_response();
        }
    };

    (StatusCode::OK, Json(res)).into_response()
}

#[instrument(level = "trace")]
async fn reset_config() -> impl IntoResponse {
    let res = match crate::reset().await {
        Ok(res) => res,
        Err(error) => {
            warn!("res from reset_config: {error:#?}");
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error:?}")).into_response();
        }
    };

    (StatusCode::OK, Json(res)).into_response()
}
