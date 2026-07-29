mod calibration;
mod camera;
mod focus;
mod macros;
mod script;
mod tilt;
mod zoom;

use ::mavlink::ardupilotmega::SERVO_OUTPUT_RAW_DATA;
use anyhow::{Context, Result};
use indexmap::IndexMap;
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;
use tracing::*;
use uuid::Uuid;

use script::ScriptHealthTracker;
use settings::MANAGER as SETTINGS_MANAGER;

use crate::{
    CameraActuators,
    api::{self, ServoChannel},
    mavlink::{self, MavlinkComponent},
};

pub static MANAGER: OnceCell<RwLock<Manager>> = OnceCell::new();

/// Serializes config apply/reset/clear. Watcher never takes this.
/// Lock order: CONFIG_APPLY → MANAGER → mavlink txn.
pub static CONFIG_APPLY: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug)]
pub struct Manager {
    pub autopilot_scripts_file: String,
    pub settings: State,
    pub(crate) script_health: ScriptHealthTracker,
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
        let actuators = self
            .actuators
            .iter()
            .map(|(uuid, actuator_settings)| (*uuid, actuator_settings.into()))
            .collect();

        let settings = &mut SETTINGS_MANAGER
            .get()
            .context("Not available")?
            .write()
            .await
            .settings;

        *settings.get_actuators_mut() = actuators;

        settings.save().await
    }
}

impl Manager {
    /// Send focus/zoom setpoints without waiting for SERVO (caller measures separately).
    ///
    /// Does not touch `MANAGER` — caller must validate the camera has actuators first.
    #[instrument(level = "debug")]
    pub async fn apply_state_setpoints(new_state: &api::ActuatorsState) -> Result<()> {
        use ::mavlink::ardupilotmega::{COMMAND_LONG_DATA, CameraZoomType, MavCmd, SetFocusType};

        if new_state.tilt.is_some() {
            if new_state.focus.is_none() && new_state.zoom.is_none() {
                return Err(anyhow::anyhow!("Tilt setpoint is not implemented"));
            }
            warn!("Ignoring unimplemented tilt setpoint; applying focus/zoom only");
        }

        let mavlink = crate::mavlink::component()?;

        if let Some(focus) = new_state.focus {
            mavlink
                .send_command(COMMAND_LONG_DATA {
                    target_system: 1,
                    target_component: 1,
                    command: MavCmd::MAV_CMD_SET_CAMERA_FOCUS,
                    confirmation: 0,
                    param1: SetFocusType::FOCUS_TYPE_RANGE as u8 as f32,
                    param2: focus,
                    param3: 0 as f32,
                    ..Default::default()
                })
                .await
                .context("Failed sending MAV_CMD_SET_CAMERA_FOCUS command")?;
        }

        if let Some(zoom) = new_state.zoom {
            mavlink
                .send_command(COMMAND_LONG_DATA {
                    target_system: 1,
                    target_component: 1,
                    command: MavCmd::MAV_CMD_SET_CAMERA_ZOOM,
                    confirmation: 0,
                    param1: CameraZoomType::ZOOM_TYPE_RANGE as u8 as f32,
                    param2: zoom,
                    param3: 0 as f32,
                    ..Default::default()
                })
                .await
                .context("Failed sending MAV_CMD_SET_CAMERA_ZOOM command")?;
        }

        Ok(())
    }

    /// Persist actuators settings without holding `MANAGER.write` across disk I/O.
    #[instrument(level = "debug")]
    pub async fn save_actuators_settings() -> Result<()> {
        let actuators = {
            let manager = MANAGER.get().context("Not available")?.read().await;
            manager
                .settings
                .actuators
                .iter()
                .map(|(uuid, actuator_settings)| (*uuid, actuator_settings.into()))
                .collect()
        };

        let settings = &mut SETTINGS_MANAGER
            .get()
            .context("Not available")?
            .write()
            .await
            .settings;

        *settings.get_actuators_mut() = actuators;
        settings.save().await
    }

    /// Apply config without holding `MANAGER.write` across MAVLink I/O.
    ///
    /// Caller must hold [`CONFIG_APPLY`]. Returns `true` if the autopilot must be
    /// rebooted before [`Self::finalize_config_after_reboot`] (enable/gain + save).
    #[instrument(level = "debug", skip(new_config))]
    pub async fn update_config(
        camera_uuid: &Uuid,
        new_config: &api::ActuatorsConfig,
        overwrite: bool,
    ) -> Result<bool> {
        let mut autopilot_reboot_required = overwrite;

        if let Some(parameters) = &new_config.parameters {
            autopilot_reboot_required |=
                Self::update_camera_parameters(camera_uuid, parameters, overwrite).await?;

            autopilot_reboot_required |=
                Self::update_script_parameters(camera_uuid, parameters, overwrite).await?;

            autopilot_reboot_required |=
                Self::update_focus_parameters(camera_uuid, parameters, overwrite).await?;

            autopilot_reboot_required |=
                Self::update_zoom_parameters(camera_uuid, parameters, overwrite).await?;

            autopilot_reboot_required |=
                Self::update_tilt_parameters(camera_uuid, parameters, overwrite).await?;
        }

        let mut reload_script = overwrite;

        if let Some(points) = &new_config.closest_points {
            reload_script |= Self::update_closest_points(camera_uuid, points, overwrite).await?;
        }
        if let Some(points) = &new_config.furthest_points {
            reload_script |= Self::update_furthest_points(camera_uuid, points, overwrite).await?;
        }

        // File write only — disk settings save is deferred to finalize / no-reboot path.
        reload_script |= Self::export_script(camera_uuid, overwrite).await?;

        autopilot_reboot_required |= crate::mavlink::component()?
            .enable_lua_script(overwrite)
            .await?;

        if reload_script && !autopilot_reboot_required {
            crate::mavlink::component()?
                .reload_lua_scripts(overwrite)
                .await?;
        }

        if autopilot_reboot_required {
            // Caller drops CONFIG_APPLY, reboots, then calls finalize_config_after_reboot.
            return Ok(true);
        }

        let force_script_params = reload_script || overwrite;
        if let Some(parameters) = &new_config.parameters {
            Self::update_script_enable(camera_uuid, parameters, force_script_params).await?;
            Self::update_script_gain(camera_uuid, parameters, force_script_params).await?;
        }

        Self::save_actuators_settings().await?;
        Ok(false)
    }

    /// Post-reboot enable/gain push and settings save. Caller must hold [`CONFIG_APPLY`].
    #[instrument(level = "debug", skip(parameters))]
    pub async fn finalize_config_after_reboot(
        camera_uuid: &Uuid,
        parameters: Option<&api::ActuatorsParametersConfig>,
    ) -> Result<()> {
        if let Some(parameters) = parameters {
            Self::update_script_enable(camera_uuid, parameters, true).await?;
            Self::update_script_gain(camera_uuid, parameters, true).await?;
        }
        Self::save_actuators_settings().await
    }

    #[instrument(level = "debug")]
    pub async fn reset_config(camera_uuid: &Uuid) -> Result<bool> {
        let actuators = CameraActuators::default();
        let config = api::ActuatorsConfig::from(&actuators);

        {
            let mut manager = MANAGER.get().context("Not available")?.write().await;
            manager.settings.actuators.insert(*camera_uuid, actuators);
        }

        Self::update_config(camera_uuid, &config, true).await
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
    let settings = State::from_settings().await?;

    if let Some(manager) = MANAGER.get() {
        let _apply = CONFIG_APPLY.lock().await;
        let mut guard = manager.write().await;
        guard.autopilot_scripts_file = autopilot_scripts_file;
        guard.settings = settings;
        return Ok(());
    }

    let mavlink =
        MavlinkComponent::try_new(mavlink_address, mavlink_system_id, mavlink_component_id).await?;
    mavlink::init_component(mavlink)?;

    let script_health = ScriptHealthTracker::default();

    MANAGER.get_or_init(|| {
        RwLock::new(Manager {
            autopilot_scripts_file,
            settings,
            script_health,
        })
    });

    crate::actuators_watch::start();

    Ok(())
}

#[instrument(level = "debug")]
pub async fn clear_saved_settings() -> Result<()> {
    settings::clear().await?;

    let _apply = CONFIG_APPLY.lock().await;
    let manager = MANAGER.get().context("Not available")?;
    let mut guard = manager.write().await;
    guard.remove_script().await?;
    guard.script_health = ScriptHealthTracker::default();
    guard.settings = State::from_settings().await?;

    Ok(())
}

pub(super) fn get_output_raw_from_channel(
    data: &SERVO_OUTPUT_RAW_DATA,
    channel: ServoChannel,
) -> Option<u16> {
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

/// Builds an [`api::ActuatorsState`] from a raw `SERVO_OUTPUT_RAW` sample.
///
/// When `enable_focus_and_zoom_correlation` is set, focus is read from the script
/// channel instead of the dedicated focus channel. Each axis is `None` when its
/// channel is unmapped.
pub(crate) fn actuators_state_from_servo(
    actuators: &CameraActuators,
    servo_output_raw: &SERVO_OUTPUT_RAW_DATA,
) -> api::ActuatorsState {
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

        get_output_raw_from_channel(servo_output_raw, channel)
            .map(|value| percentage_within_range(value, min, max))
    };

    let zoom = {
        let channel = actuators.parameters.zoom_channel;
        let min = actuators.parameters.zoom_channel_min;
        let max = actuators.parameters.zoom_channel_max;

        get_output_raw_from_channel(servo_output_raw, channel)
            .map(|value| percentage_within_range(value, min, max))
    };

    let tilt = {
        let channel = actuators.parameters.tilt_channel;
        let min = actuators.parameters.tilt_channel_min;
        let max = actuators.parameters.tilt_channel_max;

        get_output_raw_from_channel(servo_output_raw, channel)
            .map(|value| percentage_within_range(value, min, max))
    };

    api::ActuatorsState { focus, zoom, tilt }
}
