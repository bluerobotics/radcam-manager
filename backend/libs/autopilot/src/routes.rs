use axum::{Json, Router, response::IntoResponse, routing::get};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;
use ts_rs::TS;

use crate::{FocusZoomPoints, ZoomAndFocusConfig, parameters::FocusAndZoomParametersQuery};

#[derive(Debug, Default, Serialize, Deserialize, Clone, TS)]
pub struct ZoomAndFocusConfigQuery {
    pub parameters: Option<FocusAndZoomParametersQuery>,
    pub closest_points: Option<FocusZoomPoints>,
    pub furthest_points: Option<FocusZoomPoints>,
}

impl From<ZoomAndFocusConfig> for ZoomAndFocusConfigQuery {
    fn from(value: ZoomAndFocusConfig) -> Self {
        Self {
            parameters: Some(value.parameters.into()),
            closest_points: Some(value.closest_points),
            furthest_points: Some(value.furthest_points),
        }
    }
}

pub fn router() -> Router {
    Router::new()
        .route(
            "/config",
            get(get_config).post(set_config).delete(reset_config),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[instrument(level = "trace")]
async fn get_config() -> Json<ZoomAndFocusConfigQuery> {
    Json(crate::get_config().await.into())
}

#[instrument(level = "trace")]
async fn set_config(new_config: Json<ZoomAndFocusConfigQuery>) -> impl IntoResponse {
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
