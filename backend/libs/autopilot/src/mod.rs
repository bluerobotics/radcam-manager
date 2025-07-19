pub mod api;
mod manager;
mod mavlink;
pub mod parameters;
pub mod routes;

use anyhow::{Context, Result};
use axum::Json;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::*;
use uuid::Uuid;

use crate::{
    manager::{MANAGER, Manager},
    mavlink::MavlinkComponent,
    parameters::{ActuatorsParameters, CLOSEST_POINTS, FURTHEST_POINTS},
};

pub use routes::router;

#[derive(Debug, Clone)]
struct CameraActuators {
    camera_uuid: Uuid,
    config: Config,
    state: api::ActuatorsState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub parameters: ActuatorsParameters,
    pub closest_points: api::FocusZoomPoints,
    pub furthest_points: api::FocusZoomPoints,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            parameters: ActuatorsParameters::default(),
            closest_points: api::FocusZoomPoints(CLOSEST_POINTS.to_vec()),
            furthest_points: api::FocusZoomPoints(FURTHEST_POINTS.to_vec()),
        }
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

    MANAGER.get_or_init(|| {
        RwLock::new(Manager {
            mavlink,
            actuators: IndexMap::new(),
            autopilot_scripts_file,
        })
    });

    Ok(())
}

#[instrument(level = "debug")]
pub(crate) async fn control_inner(
    actuators_control: Json<api::ActuatorsControl>,
) -> Result<serde_json::Value> {
    use api::Action;

    debug!("Got control query: {actuators_control:#?}");

    let res = match &actuators_control.action {
        Action::GetActuatorsState => {
            let manager = MANAGER.get().unwrap().read().await;

            let state = &manager
                .actuators
                .get(&actuators_control.camera_uuid)
                .context("Camera's actuators not configured")?
                .state;

            serde_json::to_value(state)?
        }
        Action::SetActuatorsState(new_state) => {
            let mut manager = MANAGER.get().unwrap().write().await;

            let state = manager
                .update_state(&actuators_control.camera_uuid, new_state)
                .await?;

            serde_json::to_value(state)?
        }
        Action::GetActuatorsConfig => {
            let manager = MANAGER.get().unwrap().read().await;

            let config = &manager
                .actuators
                .get(&actuators_control.camera_uuid)
                .context("Camera's actuators not configured")?
                .config;

            serde_json::to_value(config)?
        }
        Action::SetActuatorsConfig(new_config) => {
            let mut manager = MANAGER.get().unwrap().write().await;

            manager
                .update_config(&actuators_control.camera_uuid, new_config)
                .await?;

            let config = &manager
                .actuators
                .get(&actuators_control.camera_uuid)
                .context("Camera's actuators not configured")?
                .config;

            serde_json::to_value(config)?
        }
    };

    Ok(res)
}
