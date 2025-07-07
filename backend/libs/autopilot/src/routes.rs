use axum::{Json, Router, response::IntoResponse, routing::get};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;
use ts_rs::TS;

use crate::{FocusZoomPoints, ZoomAndFocusConfig};

#[derive(Debug, Default, Serialize, Deserialize, Clone, TS)]
pub struct ApiConfig {
    pub k_focus: Option<u32>,
    pub k_zoom: Option<u32>,
    pub k_scripting1: Option<u32>,
    pub margin_gain: Option<f32>,
    pub closest_points: Option<FocusZoomPoints>,
    pub furthest_points: Option<FocusZoomPoints>,
    pub focus_channel: Option<u32>,
    pub zoom_channel: Option<u32>,
    pub custom1_channel: Option<u32>,
    pub zoom_output_pwm: Option<u32>,
    pub zoom_range: Option<u32>,
    pub zoom_scaled: Option<u32>,
}

impl From<ZoomAndFocusConfig> for ApiConfig {
    fn from(value: ZoomAndFocusConfig) -> Self {
        Self {
            k_focus: Some(value.k_focus),
            k_zoom: Some(value.k_zoom),
            k_scripting1: Some(value.k_scripting1),
            margin_gain: Some(value.margin_gain),
            closest_points: Some(value.closest_points),
            furthest_points: Some(value.furthest_points),
            focus_channel: Some(value.focus_channel),
            zoom_channel: Some(value.zoom_channel),
            custom1_channel: Some(value.custom1_channel),
            zoom_output_pwm: Some(value.zoom_output_pwm),
            zoom_range: Some(value.zoom_range),
            zoom_scaled: Some(value.zoom_scaled),
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
async fn get_config() -> Json<ApiConfig> {
    Json(crate::get_config().await.into())
}

#[instrument(level = "trace")]
async fn set_config(new_config: Json<ApiConfig>) -> impl IntoResponse {
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
