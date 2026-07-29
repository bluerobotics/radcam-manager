mod actuators_watch;
pub mod api;
mod manager;
mod mavlink;
pub mod parameters;
mod settings_translations;

use anyhow::{Context, Result};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::*;

pub use actuators_watch::{
    add_interest as add_actuators_state_interest, cache_is_fresh as actuators_cache_is_fresh,
    cached_actuators_state, interest_count as actuators_interest_count,
    remove_interest as remove_actuators_state_interest, shutdown as shutdown_actuators_stream,
    subscribe as subscribe_actuators_state,
};
pub use manager::{clear_saved_settings, init};

use crate::{
    manager::MANAGER,
    parameters::{ActuatorsParameters, CLOSEST_POINTS, FURTHEST_POINTS},
};

/// Context message when a camera has no actuators entry yet.
pub const ACTUATORS_NOT_CONFIGURED: &str = "Camera's actuators not configured";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CameraActuators {
    pub parameters: ActuatorsParameters,
    pub closest_points: api::FocusZoomPoints,
    pub furthest_points: api::FocusZoomPoints,
    pub state: api::ActuatorsState,
}

impl Default for CameraActuators {
    fn default() -> Self {
        Self {
            parameters: ActuatorsParameters::default(),
            closest_points: api::FocusZoomPoints(CLOSEST_POINTS.to_vec()),
            furthest_points: api::FocusZoomPoints(FURTHEST_POINTS.to_vec()),
            state: api::ActuatorsState::default(),
        }
    }
}

/// True when `message` (e.g. `format!("{error:?}")`) carries [`ACTUATORS_NOT_CONFIGURED`].
pub fn error_indicates_actuators_not_configured(message: &str) -> bool {
    message.contains(ACTUATORS_NOT_CONFIGURED)
}

/// Shared entry point for REST and WebSocket autopilot control requests.
#[instrument(level = "debug")]
pub async fn handle_control(actuators_control: api::ActuatorsControl) -> Result<serde_json::Value> {
    control_inner(Json(actuators_control)).await
}

#[instrument(level = "debug")]
pub(crate) async fn control_inner(
    actuators_control: Json<api::ActuatorsControl>,
) -> Result<serde_json::Value> {
    use api::Action;

    debug!("Got control query: {actuators_control:#?}");

    let res = match &actuators_control.action {
        Action::ExportLuaScript => {
            let _apply = manager::CONFIG_APPLY.lock().await;
            let reload_script = {
                let mut manager = MANAGER.get().context("Not available")?.write().await;
                manager
                    .export_script(&actuators_control.camera_uuid, true)
                    .await?
            };

            if reload_script {
                crate::mavlink::component()?
                    .reload_lua_scripts(true)
                    .await?;
            }

            let autopilot_reboot_required = crate::mavlink::component()?
                .enable_lua_script(false)
                .await?;
            if autopilot_reboot_required {
                crate::mavlink::component()?.reboot_autopilot().await?;
            }

            serde_json::to_value({})?
        }
        Action::GetActuatorsState => {
            // Prefer the SERVO watcher's cache when interest is on *and* a recent
            // sample exists. Otherwise one-shot wait so subscribe/REST are not
            // served stale defaults from disk.
            if actuators_watch::interest_count() > 0
                && actuators_watch::cache_is_fresh(actuators_control.camera_uuid)
            {
                let manager = MANAGER.get().context("Not available")?.read().await;

                let actuators = manager
                    .settings
                    .actuators
                    .get(&actuators_control.camera_uuid)
                    .context(crate::ACTUATORS_NOT_CONFIGURED)?;

                serde_json::to_value(actuators.state)?
            } else {
                // Wait for SERVO under a read lock so the watcher can still write.
                {
                    let manager = MANAGER.get().context("Not available")?.read().await;
                    let _ = manager
                        .settings
                        .actuators
                        .get(&actuators_control.camera_uuid)
                        .context(crate::ACTUATORS_NOT_CONFIGURED)?;
                }
                let age_before = actuators_watch::last_servo_age(actuators_control.camera_uuid);
                let servo_output_raw = crate::mavlink::component()?
                    .request_servo_output_raw()
                    .await
                    .context("Failed waiting for SERVO_OUTPUT_RAW_DATA message")?;
                let mut manager = MANAGER.get().context("Not available")?.write().await;
                let actuators = manager
                    .settings
                    .actuators
                    .get_mut(&actuators_control.camera_uuid)
                    .context(crate::ACTUATORS_NOT_CONFIGURED)?;
                let state = manager::actuators_state_from_servo(actuators, &servo_output_raw);
                let age_after = actuators_watch::last_servo_age(actuators_control.camera_uuid);
                // Do not clobber a newer watcher sample that landed while we waited.
                if !actuators_watch::servo_mark_advanced(age_before, age_after) {
                    actuators.state = state;
                    actuators_watch::mark_servo_from_get_state(actuators_control.camera_uuid);
                    serde_json::to_value(state)?
                } else {
                    serde_json::to_value(actuators.state)?
                }
            }
        }
        Action::SetActuatorsState(new_state) => {
            let camera_uuid = actuators_control.camera_uuid;
            let focus_was_set = new_state.focus.is_some();
            // Validate entry, then send MAVLink with no Manager lock held.
            {
                let manager = MANAGER.get().context("Not available")?.read().await;
                let _ = manager
                    .settings
                    .actuators
                    .get(&camera_uuid)
                    .context(crate::ACTUATORS_NOT_CONFIGURED)?;
            }
            manager::Manager::apply_state_setpoints(new_state).await?;
            let age_before = actuators_watch::last_servo_age(camera_uuid);
            let servo_output_raw = crate::mavlink::component()?
                .request_servo_output_raw()
                .await
                .context("Failed waiting for SERVO_OUTPUT_RAW_DATA message")?;
            let state = {
                let mut manager = MANAGER.get().context("Not available")?.write().await;
                let actuators = manager
                    .settings
                    .actuators
                    .get_mut(&camera_uuid)
                    .context(crate::ACTUATORS_NOT_CONFIGURED)?;
                let measured = manager::actuators_state_from_servo(actuators, &servo_output_raw);
                let age_after = actuators_watch::last_servo_age(camera_uuid);
                if !actuators_watch::servo_mark_advanced(age_before, age_after) {
                    actuators.state = measured;
                    actuators_watch::mark_servo_from_get_state(camera_uuid);
                    measured
                } else {
                    actuators.state
                }
            };
            // Health check waits for SERVO again — never under MANAGER.write().
            if focus_was_set {
                let enabled = {
                    let manager = MANAGER.get().context("Not available")?.read().await;
                    manager
                        .settings
                        .actuators
                        .get(&camera_uuid)
                        .is_some_and(|a| a.parameters.enable_focus_and_zoom_correlation)
                };
                if enabled {
                    let health_servo = crate::mavlink::component()?
                        .request_servo_output_raw()
                        .await
                        .ok();
                    if let Some(health_servo) = health_servo {
                        let needs_reload = {
                            let mut manager = MANAGER.get().context("Not available")?.write().await;
                            manager.apply_focus_script_health_sample(&camera_uuid, &health_servo)
                        };
                        if needs_reload {
                            warn!("Attempting Lua script reload due to stale focus output");
                            if let Err(error) =
                                crate::mavlink::component()?.reload_lua_scripts(true).await
                            {
                                error!("Failed to reload Lua scripts: {error:?}");
                            }
                        }
                    }
                }
            }
            serde_json::to_value(state)?
        }
        Action::GetActuatorsConfig => {
            let manager = MANAGER.get().context("Not available")?.read().await;

            let config: &api::ActuatorsConfig = &manager
                .settings
                .actuators
                .get(&actuators_control.camera_uuid)
                .context(crate::ACTUATORS_NOT_CONFIGURED)?
                .into();

            serde_json::to_value(config)?
        }
        Action::GetActuatorsDefaultConfig => {
            let config = api::ActuatorsConfig::from(&CameraActuators::default());

            serde_json::to_value(config)?
        }
        Action::SetActuatorsConfig(new_config) => {
            let _apply = manager::CONFIG_APPLY.lock().await;
            let mut manager = MANAGER.get().context("Not available")?.write().await;
            let mut new_config = new_config.to_owned();

            let base_config = &manager
                .settings
                .actuators
                .get(&actuators_control.camera_uuid)
                .map(api::ActuatorsConfig::from)
                .unwrap_or(api::ActuatorsConfig::from(&CameraActuators::default()));

            new_config = merge_struct::merge(base_config, &new_config.clone())
                .context("Failing to merge structs")?;

            // ponytail: still holds MANAGER.write across param I/O — CONFIG_APPLY
            // serializes mutators; full snapshot-I/O-commit of hand-written channel
            // swaps is the upgrade path.
            manager
                .update_config(&actuators_control.camera_uuid, &new_config, false)
                .await?;

            let config: &api::ActuatorsConfig = &manager
                .settings
                .actuators
                .get(&actuators_control.camera_uuid)
                .context(crate::ACTUATORS_NOT_CONFIGURED)?
                .into();

            serde_json::to_value(config)?
        }
        Action::ResetActuatorsConfig => {
            let _apply = manager::CONFIG_APPLY.lock().await;
            let mut manager = MANAGER.get().context("Not available")?.write().await;

            manager.reset_config(&actuators_control.camera_uuid).await?;

            let config: &api::ActuatorsConfig = &manager
                .settings
                .actuators
                .get(&actuators_control.camera_uuid)
                .context(crate::ACTUATORS_NOT_CONFIGURED)?
                .into();

            serde_json::to_value(config)?
        }
    };

    Ok(res)
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;

    use super::{ACTUATORS_NOT_CONFIGURED, error_indicates_actuators_not_configured};

    #[test]
    fn actuators_not_configured_message_is_stable() {
        let error = anyhow!("missing entry").context(ACTUATORS_NOT_CONFIGURED);
        assert!(error_indicates_actuators_not_configured(&format!(
            "{error:?}"
        )));
        assert!(!error_indicates_actuators_not_configured(
            "Camera actuators unavailable"
        ));
    }
}
