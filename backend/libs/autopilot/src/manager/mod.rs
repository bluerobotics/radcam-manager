mod calibration;
mod camera;
mod focus;
mod macros;
mod script;
mod tilt;
mod zoom;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use mavlink::ardupilotmega::SERVO_OUTPUT_RAW_DATA;
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;
use tracing::*;
use uuid::Uuid;

use settings::MANAGER as SETTINGS_MANAGER;

use crate::{
    CameraActuators,
    api::{self, ServoChannel},
    mavlink::MavlinkComponent,
};

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
    #[instrument(level = "debug")]
    pub async fn from_settings() -> Result<Self> {
        let settings = &SETTINGS_MANAGER
            .get()
            .context("Not available")?
            .read()
            .await
            .settings;

        let actuators = settings
            .get_actuators()
            .iter()
            .map(|(uuid, actuator_settings)| (*uuid, CameraActuators::from(actuator_settings)))
            .collect();

        Ok(Self { actuators })
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn save(&self) -> Result<()> {
        let settings = &mut SETTINGS_MANAGER
            .get()
            .context("Not available")?
            .write()
            .await
            .settings;

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
    pub async fn get_state(&mut self, camera_uuid: &Uuid) -> Result<api::ActuatorsState> {
        let actuators = self
            .settings
            .actuators
            .get_mut(camera_uuid)
            .context("Camera's actuators not configured")?;

        let servo_output_raw = self
            .mavlink
            .request_servo_output_raw()
            .await
            .context("Failed waiting for SERVO_OUTPUT_RAW_DATA message")?;

        let focus = {
            let (channel, min, max) = if actuators.parameters.enable_focus_and_zoom_correlation {
                (
                    actuators.parameters.script_channel,
                    actuators.parameters.script_channel_min,
                    actuators.parameters.script_channel_max,
                )
            } else {
                (
                    actuators.parameters.focus_channel,
                    actuators.parameters.focus_channel_min,
                    actuators.parameters.focus_channel_max,
                )
            };

            get_output_raw_from_channel(&servo_output_raw, channel)
                .map(|value| percentage_within_range(value, min, max))
        };

        let zoom = {
            let channel = actuators.parameters.zoom_channel;
            let min = actuators.parameters.zoom_channel_min;
            let max = actuators.parameters.zoom_channel_max;

            get_output_raw_from_channel(&servo_output_raw, channel)
                .map(|value| percentage_within_range(value, min, max))
        };

        let tilt = {
            let channel = actuators.parameters.tilt_channel;
            let min = actuators.parameters.tilt_channel_min;
            let max = actuators.parameters.tilt_channel_max;

            get_output_raw_from_channel(&servo_output_raw, channel)
                .map(|value| percentage_within_range(value, min, max))
        };

        actuators.state = api::ActuatorsState { focus, zoom, tilt };

        Ok(actuators.state)
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn update_state(
        &mut self,
        camera_uuid: &Uuid,
        new_state: &api::ActuatorsState,
    ) -> Result<api::ActuatorsState> {
        use ::mavlink::ardupilotmega::{COMMAND_LONG_DATA, CameraZoomType, MavCmd, SetFocusType};

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
        }

        let _ = self.get_state(camera_uuid).await;

        Ok(*new_state)
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn update_config(
        &mut self,
        camera_uuid: &Uuid,
        new_config: &api::ActuatorsConfig,
        overwrite: bool,
    ) -> Result<()> {
        let mut autopilot_reboot_required = overwrite;

        // Parameters update
        if let Some(parameters) = &new_config.parameters {
            autopilot_reboot_required |= self
                .update_camera_parameters(camera_uuid, parameters, overwrite)
                .await?;

            autopilot_reboot_required |= self
                .update_script_parameters(camera_uuid, parameters, overwrite)
                .await?;

            autopilot_reboot_required |= self
                .update_focus_parameters(camera_uuid, parameters, overwrite)
                .await?;

            autopilot_reboot_required |= self
                .update_zoom_parameters(camera_uuid, parameters, overwrite)
                .await?;

            autopilot_reboot_required |= self
                .update_tilt_parameters(camera_uuid, parameters, overwrite)
                .await?;
        }

        let mut reload_script = overwrite;

        // Callibration update
        if let Some(points) = &new_config.closest_points {
            reload_script |= self
                .update_closest_points(camera_uuid, points, overwrite)
                .await?;
        }
        if let Some(points) = &new_config.furthest_points {
            reload_script |= self
                .update_furthest_points(camera_uuid, points, overwrite)
                .await?;
        }

        reload_script |= self.export_script(camera_uuid, overwrite).await?;

        autopilot_reboot_required |= self.mavlink.enable_lua_script(overwrite).await?;

        if reload_script && !autopilot_reboot_required {
            self.mavlink.reload_lua_scripts(overwrite).await?;
        }

        if autopilot_reboot_required {
            self.mavlink.reboot_autopilot().await?;
        }

        if let Some(parameters) = &new_config.parameters {
            self.update_script_enable(camera_uuid, parameters, true)
                .await?;

            self.update_script_gain(camera_uuid, parameters, true)
                .await?;
        }

        self.settings.save().await?;

        Ok(())
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn reset_config(&mut self, camera_uuid: &Uuid) -> Result<()> {
        let actuators = CameraActuators::default();
        let config = api::ActuatorsConfig::from(&actuators);

        self.settings.actuators.insert(*camera_uuid, actuators);

        self.update_config(camera_uuid, &config, true).await
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
        MavlinkComponent::try_new(mavlink_address, mavlink_system_id, mavlink_component_id).await?;

    let settings = State::from_settings().await?;

    MANAGER.get_or_init(|| {
        RwLock::new(Manager {
            mavlink,
            autopilot_scripts_file,
            settings,
        })
    });

    Ok(())
}

fn get_output_raw_from_channel(data: &SERVO_OUTPUT_RAW_DATA, channel: ServoChannel) -> Option<u16> {
    match channel {
        ServoChannel::SERVO1 => Some(data.servo1_raw),
        ServoChannel::SERVO2 => Some(data.servo2_raw),
        ServoChannel::SERVO3 => Some(data.servo3_raw),
        ServoChannel::SERVO4 => Some(data.servo4_raw),
        ServoChannel::SERVO5 => Some(data.servo5_raw),
        ServoChannel::SERVO6 => Some(data.servo6_raw),
        ServoChannel::SERVO7 => Some(data.servo7_raw),
        ServoChannel::SERVO8 => Some(data.servo8_raw),
        ServoChannel::SERVO9 => Some(data.servo9_raw),
        ServoChannel::SERVO10 => Some(data.servo10_raw),
        ServoChannel::SERVO11 => Some(data.servo11_raw),
        ServoChannel::SERVO12 => Some(data.servo12_raw),
        ServoChannel::SERVO13 => Some(data.servo13_raw),
        ServoChannel::SERVO14 => Some(data.servo14_raw),
        ServoChannel::SERVO15 => Some(data.servo15_raw),
        ServoChannel::SERVO16 => Some(data.servo16_raw),
        _ => None,
    }
}

fn percentage_within_range(value: u16, min: u16, max: u16) -> f32 {
    if max == min {
        return 0.0;
    }
    let clamped = value.clamp(min, max);
    (100.0 * ((clamped - min) as f32 / (max - min) as f32)).round()
}
