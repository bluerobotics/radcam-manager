use axum::{Json, Router, response::IntoResponse, routing::get};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;
use ts_rs::TS;
use uuid::Uuid;

use crate::{FocusZoomPoints, ZoomAndFocusConfig, parameters::FocusAndZoomParametersConfig};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
// #[tsync] // FIXME: Disabled for now, see https://github.com/Wulf/tsync/issues/58
pub struct ZoomAndFocusControl {
    #[ts(as = "String")]
    pub camera_uuid: Uuid,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "action", content = "json")]
// #[tsync] // FIXME: Disabled for now, see https://github.com/Wulf/tsync/issues/58
pub enum Action {
    #[serde(rename = "getZoomAndFocus")]
    GetZoomAndFocus,
    #[serde(rename = "setZoomAndFocus")]
    SetZoomAndFocus(SetZoomAndFocus),
    #[serde(rename = "getZoomAndFocusConfig")]
    GetZoomAndFocusConfig,
    #[serde(rename = "setZoomAndFocusConfig")]
    SetZoomAndFocusConfig(SetZoomAndFocusConfig),
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, TS)]
pub struct SetZoomAndFocus {
    pub focus: Option<u16>,
    pub zoom: Option<u16>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, TS)]
pub struct SetZoomAndFocusConfig {
    pub parameters: Option<FocusAndZoomParametersConfig>,
    pub closest_points: Option<FocusZoomPoints>,
    pub furthest_points: Option<FocusZoomPoints>,
}

impl From<ZoomAndFocusConfig> for SetZoomAndFocusConfig {
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
async fn get_config() -> Json<SetZoomAndFocusConfig> {
    Json(crate::get_config().await.into())
}

#[instrument(level = "trace")]
async fn set_config(new_config: Json<SetZoomAndFocusConfig>) -> impl IntoResponse {
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
