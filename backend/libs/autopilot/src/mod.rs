pub mod api;
mod manager;
mod mavlink;
pub mod parameters;
pub mod routes;
mod settings_translations;

use anyhow::{Context, Result};
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::*;

pub use manager::init;

use crate::{
    manager::MANAGER,
    parameters::{ActuatorsParameters, CLOSEST_POINTS, FURTHEST_POINTS},
};

pub use routes::router;

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

#[instrument(level = "debug")]
pub(crate) async fn control_inner(
    actuators_control: Json<api::ActuatorsControl>,
) -> Result<serde_json::Value> {
    use api::Action;

    debug!("Got control query: {actuators_control:#?}");

    let res = match &actuators_control.action {
        Action::ExportLuaScript => {
            let mut manager = MANAGER.get().context("Not available")?.write().await;

            let reload_script = manager
                .export_script(&actuators_control.camera_uuid, true)
                .await?;

            if reload_script {
                manager.mavlink.reload_lua_scripts(true).await?;
            }

            let autopilot_reboot_required = manager.mavlink.enable_lua_script(false).await?;
            if autopilot_reboot_required {
                manager.mavlink.reboot_autopilot().await?;
            }

            serde_json::to_value({})?
        }
        Action::GetActuatorsState => {
            let mut manager = MANAGER.get().context("Not available")?.write().await;

            let state = manager.get_state(&actuators_control.camera_uuid).await?;

            serde_json::to_value(state)?
        }
        Action::SetActuatorsState(new_state) => {
            let mut manager = MANAGER.get().context("Not available")?.write().await;

            let state = manager
                .update_state(&actuators_control.camera_uuid, new_state)
                .await?;

            serde_json::to_value(state)?
        }
        Action::GetActuatorsConfig => {
            let manager = MANAGER.get().context("Not available")?.read().await;

            let config: &api::ActuatorsConfig = &manager
                .settings
                .actuators
                .get(&actuators_control.camera_uuid)
                .context("Camera's actuators not configured")?
                .into();

            serde_json::to_value(config)?
        }
        Action::SetActuatorsConfig(new_config) => {
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

            manager
                .update_config(&actuators_control.camera_uuid, &new_config, false)
                .await?;

            let config: &api::ActuatorsConfig = &manager
                .settings
                .actuators
                .get(&actuators_control.camera_uuid)
                .context("Camera's actuators not configured")?
                .into();

            serde_json::to_value(config)?
        }
        Action::ResetActuatorsConfig => {
            let mut manager = MANAGER.get().context("Not available")?.write().await;

            manager.reset_config(&actuators_control.camera_uuid).await?;

            let config: &api::ActuatorsConfig = &manager
                .settings
                .actuators
                .get(&actuators_control.camera_uuid)
                .context("Camera's actuators not configured")?
                .into();

            serde_json::to_value(config)?
        }
    };

    Ok(res)
}
