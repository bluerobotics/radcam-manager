mod calibration;
mod focus;
mod macros;
mod script;
mod tilt;
mod zoom;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;
use tracing::*;
use uuid::Uuid;

use crate::{
    CameraActuators, Config, api, manager::script::export_script, mavlink::MavlinkComponent,
};

pub static MANAGER: OnceCell<RwLock<Manager>> = OnceCell::new();

#[derive(Debug)]
pub struct Manager {
    pub mavlink: MavlinkComponent,
    pub actuators: IndexMap<Uuid, CameraActuators>,
    pub autopilot_scripts_file: String,
}

impl Manager {
    #[instrument(level = "debug", skip(self))]
    pub async fn update_state(
        &mut self,
        camera_uuid: &Uuid,
        new_state: &api::ActuatorsState,
    ) -> Result<api::ActuatorsState> {
        use ::mavlink::ardupilotmega::{COMMAND_LONG_DATA, CameraZoomType, MavCmd, SetFocusType};

        let mut current_state = api::ActuatorsState::default();

        if let Some(focus) = new_state.focus {
            self.mavlink
                .send_command(COMMAND_LONG_DATA {
                    target_system: 1,
                    target_component: 1,
                    command: MavCmd::MAV_CMD_SET_CAMERA_FOCUS,
                    confirmation: 0,
                    param1: SetFocusType::FOCUS_TYPE_RANGE as u8 as f32,
                    param2: focus as f32,
                    param3: 0 as f32, // autopilot cameras
                    ..Default::default()
                })
                .await
                .context("Failed sending MAV_CMD_SET_CAMERA_FOCUS command")?;

            let state = self
                .mavlink
                .wait_camera_settings()
                .await
                .context("Failed waiting for CAMERA_SETTINGS after MAV_CMD_SET_CAMERA_FOCUS")?;

            current_state.focus = none_if_nan(state.focusLevel);
            current_state.zoom = none_if_nan(state.zoomLevel);
        }

        if let Some(zoom) = new_state.zoom {
            self.mavlink
                .send_command(COMMAND_LONG_DATA {
                    target_system: 1,
                    target_component: 1,
                    command: MavCmd::MAV_CMD_SET_CAMERA_ZOOM,
                    confirmation: 0,
                    param1: CameraZoomType::ZOOM_TYPE_RANGE as u8 as f32,
                    param2: zoom as f32,
                    param3: 0 as f32, // autopilot cameras
                    ..Default::default()
                })
                .await
                .context("Failed sending MAV_CMD_SET_CAMERA_ZOOM command")?;

            let state = self
                .mavlink
                .wait_camera_settings()
                .await
                .context("Failed waiting for CAMERA_SETTINGS after MAV_CMD_SET_CAMERA_ZOOM")?;

            current_state.focus = none_if_nan(state.focusLevel);
            current_state.zoom = none_if_nan(state.zoomLevel);
        }

        if let Some(_tilt) = new_state.tilt {
            warn!("TILT NOT IMPLEMENTED!");
        }

        self.actuators
            .entry(*camera_uuid)
            .and_modify(|v| v.state = current_state.clone());

        Ok(current_state)
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn update_config(
        &mut self,
        camera_uuid: &Uuid,
        new_config: &api::ActuatorsConfig,
    ) -> Result<()> {
        let mut new_config = new_config;
        let default_config = api::ActuatorsConfig::from(Config::default());
        let mut autopilot_reboot_required = false;

        // If everything is empty, set default
        if new_config.parameters.is_none()
            && new_config.closest_points.is_none()
            && new_config.furthest_points.is_none()
        {
            debug!("Setting to default: {default_config:?}");
            new_config = &default_config;
        }

        // Parameters update
        if let Some(parameters) = &new_config.parameters {
            autopilot_reboot_required |= self
                .update_script_parameters(camera_uuid, parameters)
                .await?;

            autopilot_reboot_required |= self
                .update_focus_parameters(camera_uuid, parameters)
                .await?;

            autopilot_reboot_required |=
                self.update_zoom_parameters(camera_uuid, parameters).await?;

            autopilot_reboot_required |=
                self.update_tilt_parameters(camera_uuid, parameters).await?;
        }

        // Callibration update
        if let Some(points) = &new_config.closest_points {
            autopilot_reboot_required |= self.update_closest_points(camera_uuid, points).await?;
        }
        if let Some(points) = &new_config.furthest_points {
            autopilot_reboot_required |= self.update_furthest_points(camera_uuid, points).await?;
        }

        if autopilot_reboot_required {
            export_script(&self.autopilot_scripts_file).await?;

            self.mavlink.enable_lua_script().await?;

            self.mavlink.restart_autopilot().await?;
        }

        Ok(())
    }
}

fn none_if_nan(value: f32) -> Option<f32> {
    if value.is_nan() { None } else { Some(value) }
}
