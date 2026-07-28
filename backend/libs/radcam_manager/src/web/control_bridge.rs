//! Single entry point for camera and autopilot mutations, so every caller
//! (REST and WebSocket alike) produces the same UI overlay and state updates.

use autopilot::api::ActuatorsControl;
use radcam_commands::CameraControl;
use serde_json::Value;

use crate::web::{camera_state, camera_ui};

/// Run a camera control, driving the shared UI overlay and state stream.
#[tracing::instrument(level = "debug", skip_all, fields(%camera_control.camera_uuid))]
pub(crate) async fn camera_control(camera_control: CameraControl) -> Result<Value, String> {
    let camera_uuid = camera_control.camera_uuid;
    if mcm_client::get_camera(&camera_uuid).await.is_none() {
        return Err("unknown camera".to_string());
    }
    let action = camera_control.action.clone();
    camera_ui::start_camera_action(camera_uuid, &action);

    match radcam_commands::handle_control(camera_control).await {
        Ok(value) => {
            camera_ui::finish_camera_action(camera_uuid, &action);
            camera_state::emit_camera_control_update(camera_uuid, &action, &value);
            Ok(value)
        }
        Err(error) => {
            let error = format!("{error:?}");
            camera_ui::fail_camera_action(camera_uuid, &action, &error);
            Err(error)
        }
    }
}

/// Run an autopilot control, driving the shared UI overlay and state stream.
#[tracing::instrument(level = "debug", skip_all, fields(%actuators_control.camera_uuid))]
pub(crate) async fn autopilot_control(
    actuators_control: ActuatorsControl,
) -> Result<Value, String> {
    let camera_uuid = actuators_control.camera_uuid;
    if mcm_client::get_camera(&camera_uuid).await.is_none() {
        return Err("unknown camera".to_string());
    }
    let action = actuators_control.action.clone();
    camera_ui::start_autopilot_action(camera_uuid, &action);

    match autopilot::handle_control(actuators_control).await {
        Ok(value) => {
            camera_ui::finish_autopilot_action(camera_uuid, &action);
            camera_state::emit_autopilot_control_update(camera_uuid, &action, &value);
            Ok(value)
        }
        Err(error) => {
            let error = format!("{error:?}");
            camera_ui::fail_autopilot_action(camera_uuid, &action, &error);
            Err(error)
        }
    }
}
