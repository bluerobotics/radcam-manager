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

use settings::MANAGER as SETTINGS_MANAGER;

use crate::{CameraActuators, api, manager::script::export_script, mavlink::MavlinkComponent};

pub static MANAGER: OnceCell<RwLock<Manager>> = OnceCell::new();

#[derive(Debug)]
pub struct Manager {
    pub mavlink: MavlinkComponent,
    pub autopilot_scripts_file: String,
    pub settings: State,
}

#[derive(Debug)]
pub struct State {
    pub actuators: IndexMap<Uuid, CameraActuators>,
}

impl State {
    pub async fn from_settings() -> Self {
        let settings = &SETTINGS_MANAGER.get().unwrap().read().await.settings;

        let actuators = settings
            .get_actuators()
            .iter()
            .map(|(uuid, actuator_settings)| (*uuid, CameraActuators::from(actuator_settings)))
            .collect();

        Self { actuators }
    }

    pub async fn save(&self) -> Result<()> {
        let settings = &mut SETTINGS_MANAGER.get().unwrap().write().await.settings;

        let actuators = self
            .actuators
            .iter()
            .map(|(uuid, actuator_settings)| (*uuid, actuator_settings.into()))
            .collect();

        *settings.get_actuators_mut() = actuators;

        settings.save().await
    }
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
                    param2: focus,
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
                    param2: zoom,
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

        self.settings
            .actuators
            .entry(*camera_uuid)
            .and_modify(|v| v.state = current_state);

        self.settings.save().await?;

        Ok(current_state)
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn update_config(
        &mut self,
        camera_uuid: &Uuid,
        new_config: &api::ActuatorsConfig,
    ) -> Result<()> {
        let mut new_config = new_config;
        let default_config = api::ActuatorsConfig::from(&CameraActuators::default());
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

        autopilot_reboot_required |= export_script(&self.autopilot_scripts_file).await?;
        autopilot_reboot_required |= self.mavlink.enable_lua_script().await?;

        if autopilot_reboot_required {
            self.mavlink.restart_autopilot().await?;
        }

        self.settings.save().await?;

        Ok(())
    }

    pub async fn export_script(&mut self) -> Result<bool> {
        let mut autopilot_reboot_required = false;

        autopilot_reboot_required |= export_script(&self.autopilot_scripts_file).await?;
        autopilot_reboot_required |= self.mavlink.enable_lua_script().await?;

        if autopilot_reboot_required {
            self.mavlink.restart_autopilot().await?;
        }

        self.settings.save().await?;

        Ok(autopilot_reboot_required)
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn reset_config(&mut self, camera_uuid: &Uuid) -> Result<()> {
        let default_config = api::ActuatorsConfig::from(&CameraActuators::default());
        debug!("Setting to default: {default_config:?}");

        self.settings
            .actuators
            .insert(*camera_uuid, ActuatorsConfig)?;

        self.update_config(camera_uuid, &default_config).await
    }
}

/// Constructs our manager, Should be done inside main
#[instrument(level = "debug")]
pub async fn init(
    autopilot_scripts_file: String,
    mavlink_address: String,
    mavlink_system_id: u8,
    mavlink_component_id: u8,
) -> Result<()> {
    let mavlink =
        MavlinkComponent::new(mavlink_address, mavlink_system_id, mavlink_component_id).await;

    let settings = State::from_settings().await;

    MANAGER.get_or_init(|| {
        RwLock::new(Manager {
            mavlink,
            autopilot_scripts_file,
            settings,
        })
    });

    Ok(())
}

fn none_if_nan(value: f32) -> Option<f32> {
    if value.is_nan() { None } else { Some(value) }
}
