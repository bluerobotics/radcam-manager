//! Wire types and protocol envelopes shared by the RadCam Manager backend and the
//! generated TypeScript bindings.
//!
//! This crate must stay a dependency-light leaf: it must not depend on
//! `radcam_manager` (or other application crates). `bindings_generator` exports these
//! types into `frontend/src/bindings/radcam_api.d.ts`.
//!
//! That file is the source of truth for the WebSocket *envelopes* and shared UI state
//! (`WsRequest` / `WsResponse` / `WsEvent`, `CameraStateEvent`, `CameraUiState`,
//! `ConnectionStats`). Camera and autopilot *payload* shapes live in `radcam.ts` /
//! `autopilot.d.ts` (from `radcam_commands` and `autopilot::api`); several
//! `CameraStateEvent` fields remain `serde_json::Value` / TypeScript `unknown` for
//! that reason.

use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
use uuid::Uuid;

/// Shared UI overlay state synchronized to every subscribed frontend.
#[derive(Debug, Clone, Serialize, Default, PartialEq, TS)]
pub struct CameraUiState {
    /// True while a long-running backend action is in progress.
    pub loading: bool,
    /// Human-readable loading message.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub loading_message: Option<String>,
    /// True while waiting for the camera to come back after reboot.
    pub rebooting: bool,
    /// Modal error dialog message, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error_dialog: Option<String>,
    /// Transient warning toast message, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub warning_toast: Option<String>,
}

/// Partial camera state pushed to subscribed WebSocket clients.
#[derive(Debug, Clone, Serialize, Default, TS)]
pub struct CameraStateEvent {
    /// Camera this event belongs to.
    #[ts(as = "String")]
    pub camera_uuid: Uuid,
    /// Default actuator configuration, included on initial subscribe snapshots.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    pub actuators_default_config: Option<serde_json::Value>,
    /// Current actuator configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    pub actuators_config: Option<serde_json::Value>,
    /// Current actuator positions.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    pub actuators_state: Option<serde_json::Value>,
    /// Whether actuators are configured for this camera.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub actuators_configured: Option<bool>,
    /// Current video encoder parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    pub video_parameters: Option<serde_json::Value>,
    /// Current base image parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    pub base_parameters: Option<serde_json::Value>,
    /// Current advanced image parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    pub advanced_parameters: Option<serde_json::Value>,
    /// Shared UI overlay state for this camera.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub ui: Option<CameraUiState>,
}

/// Per-connection and aggregate bandwidth statistics for the connection icon tooltip.
#[derive(Debug, Clone, Serialize, TS)]
pub struct ConnectionStats {
    /// Always true for live connections; false if the id is unknown.
    pub connected: bool,
    /// When this WebSocket connected.
    #[ts(type = "string")]
    pub since: DateTime<Utc>,
    /// Number of currently open WebSocket clients.
    pub clients_connected: usize,
    /// Minute-average upload kbps for this connection (client → server).
    pub this_upload_kbps: f64,
    /// Minute-average download kbps for this connection (server → client).
    pub this_download_kbps: f64,
    /// Minute-average upload kbps summed across all connections.
    pub total_upload_kbps: f64,
    /// Minute-average download kbps summed across all connections.
    pub total_download_kbps: f64,
}

/// Request-response call sent by a client over the WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct WsRequest {
    /// Client-chosen correlation id echoed back in the [`WsResponse`].
    pub id: u32,
    /// HTTP-like verb, e.g. `GET` or `POST`.
    pub method: String,
    /// Route being called, e.g. `/camera/list`.
    pub path: String,
    /// Optional payload, required by `POST` routes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional, type = "unknown")]
    pub body: Option<Value>,
}

/// Fire-and-forget message sent by a client over the WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsClientMessage {
    /// Start receiving [`CameraStateEvent`]s for a camera.
    Subscribe {
        /// Camera to subscribe to.
        #[ts(as = "String")]
        camera_uuid: Uuid,
    },
    /// Stop receiving [`CameraStateEvent`]s for a camera.
    Unsubscribe {
        /// Camera to unsubscribe from.
        #[ts(as = "String")]
        camera_uuid: Uuid,
    },
    /// Clear one of the shared [`CameraUiState`] overlays.
    UiDismiss {
        /// Camera whose overlay should be cleared.
        #[ts(as = "String")]
        camera_uuid: Uuid,
        /// Overlay to clear.
        field: UiDismissField,
    },
}

/// Shared [`CameraUiState`] overlay a client can dismiss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum UiDismissField {
    /// The modal error dialog.
    ErrorDialog,
    /// The transient warning toast.
    WarningToast,
}

/// Answer to a [`WsRequest`], correlated by `id`.
///
/// `response_type` is a `String` (not an enum) so the TypeScript binding can pin it
/// to the literal `"response"` via `ts_rs` without a tagged enum.
#[derive(Debug, Clone, Serialize, TS)]
pub struct WsResponse {
    /// Message discriminator, always `response`.
    #[serde(rename = "type")]
    #[ts(type = "\"response\"")]
    pub response_type: String,
    /// Correlation id copied from the [`WsRequest`].
    pub id: u32,
    /// HTTP-like status code.
    pub status: u16,
    /// Result payload, or an error description for non-2xx statuses.
    #[ts(type = "unknown")]
    pub body: Value,
}

/// Unsolicited server push, e.g. `camera/state` or `connection/stats`.
///
/// `event_type` is a `String` (not an enum) so the TypeScript binding can pin it
/// to the literal `"event"` via `ts_rs` without a tagged enum.
#[derive(Debug, Clone, Serialize, TS)]
pub struct WsEvent {
    /// Message discriminator, always `event`.
    #[serde(rename = "type")]
    #[ts(type = "\"event\"")]
    pub event_type: String,
    /// Event name, e.g. `camera/list`.
    pub event: String,
    /// Event payload.
    #[ts(type = "unknown")]
    pub body: Value,
}

impl WsResponse {
    /// Builds a `response` message for the given request id.
    pub fn new(id: u32, status: u16, body: Value) -> Self {
        Self {
            response_type: "response".to_string(),
            id,
            status,
            body,
        }
    }
}

impl WsEvent {
    /// Builds an `event` message for the given event name.
    pub fn new(event: impl Into<String>, body: Value) -> Self {
        Self {
            event_type: "event".to_string(),
            event: event.into(),
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_messages_keep_the_wire_shape() {
        let uuid = "5b1c6a6c-3f5e-4d1e-9a3f-8a7d0e2b1c34";

        let message: WsClientMessage =
            serde_json::from_str(&format!(r#"{{"type":"subscribe","camera_uuid":"{uuid}"}}"#))
                .unwrap();
        assert!(matches!(message, WsClientMessage::Subscribe { .. }));

        let message: WsClientMessage = serde_json::from_str(&format!(
            r#"{{"type":"unsubscribe","camera_uuid":"{uuid}"}}"#
        ))
        .unwrap();
        assert!(matches!(message, WsClientMessage::Unsubscribe { .. }));

        let message: WsClientMessage = serde_json::from_str(&format!(
            r#"{{"type":"ui_dismiss","camera_uuid":"{uuid}","field":"warning_toast"}}"#
        ))
        .unwrap();
        let WsClientMessage::UiDismiss { field, .. } = message else {
            panic!("expected a ui_dismiss message");
        };
        assert_eq!(field, UiDismissField::WarningToast);
    }

    #[test]
    fn requests_accept_a_missing_body() {
        let request: WsRequest =
            serde_json::from_str(r#"{"id":7,"method":"GET","path":"/camera/list"}"#).unwrap();
        assert!(request.body.is_none());
    }

    #[test]
    fn envelopes_keep_the_wire_shape() {
        assert_eq!(
            serde_json::to_string(&WsResponse::new(7, 200, Value::Null)).unwrap(),
            r#"{"type":"response","id":7,"status":200,"body":null}"#
        );
        assert_eq!(
            serde_json::to_string(&WsEvent::new("camera/state", Value::Null)).unwrap(),
            r#"{"type":"event","event":"camera/state","body":null}"#
        );
    }
}
