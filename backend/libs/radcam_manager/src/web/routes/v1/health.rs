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

#[cfg(test)]
mod tests {
    use radcam_api::{McmHealth, WsEvent};

    use super::*;

    #[tokio::test]
    async fn get_health_json_shape() {
        let health = connectivity::system_health().await;
        let json = serde_json::to_value(&health).expect("serialize SystemHealth");

        assert!(json.get("expected_missing").unwrap().is_array());
        assert!(json.get("diagnostics").unwrap().is_object());
        assert!(json.get("cameras_discovered").unwrap().is_number());

        if health.mcm == McmHealth::Online {
            assert!(json.get("mcm_detail").is_none());
        }
        if health.autopilot == radcam_api::AutopilotHealth::Online {
            assert!(json.get("autopilot_detail").is_none());
        }
    }

    #[tokio::test]
    async fn system_health_event_wire_shape() {
        let health = connectivity::system_health().await;
        let body = serde_json::to_value(&health).expect("serialize SystemHealth");
        let text =
            serde_json::to_string(&WsEvent::new("system/health", body)).expect("serialize event");

        let value: serde_json::Value = serde_json::from_str(&text).expect("parse event");
        assert_eq!(value["type"], "event");
        assert_eq!(value["event"], "system/health");
        assert!(value["body"]["mcm"].is_string());
        assert!(value["body"]["expected_missing"].is_array());
    }
}
