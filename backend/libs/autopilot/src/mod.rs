mod defaults;
mod mavlink;
mod parameters;
pub mod routes;
mod script;
mod settings;

use ::mavlink::{
    MessageData,
    ardupilotmega::{
        AUTOPILOT_VERSION_DATA, MavMessage, MavProtocolCapability, PARAM_SET_DATA, PARAM_VALUE_DATA,
    },
};
pub use routes::{ApiConfig, router};

use anyhow::{Result, anyhow};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, task::JoinHandle};
use tracing::*;
use ts_rs::TS;

use crate::{
    mavlink::MavlinkComponent,
    parameters::FocusAndZoomParameters,
    script::{generate_lua_script, validate_lua},
    settings::{read_settings, write_settings},
};

static MANAGER: OnceCell<RwLock<Manager>> = OnceCell::new();

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZoomAndFocusConfig {
    pub k_focus: u32,
    pub k_zoom: u32,
    pub k_scripting1: u32,
    pub margin_gain: f32,
    pub closest_points: FocusZoomPoints,
    pub furthest_points: FocusZoomPoints,
    pub focus_channel: u32,
    pub zoom_channel: u32,
    pub custom1_channel: u32,
    pub zoom_output_pwm: u32,
    pub zoom_range: u32,
    pub zoom_scaled: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
pub struct FocusZoomPoints(Vec<FocusZoomPoint>);
impl FocusZoomPoints {
    pub fn to_lua(&self) -> String {
        let entries: Vec<String> = self
            .0
            .iter()
            .map(|point| format!("    {{zoom = {}, focus = {}}}", point.zoom, point.focus))
            .collect();

        format!("{{\n{}\n}}", entries.join(",\n"))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, TS)]
pub struct FocusZoomPoint {
    zoom: u32,
    focus: u32,
}

#[derive(Debug)]
struct Manager {
    config: ZoomAndFocusConfig,
    parameters: FocusAndZoomParameters,
    mavlink: MavlinkComponent,
    settings_file: String,
    autopilot_scripts_file: String,
    _task_handler: JoinHandle<()>,
}

/// Constructs our manager, Should be done inside main
#[instrument(level = "debug")]
pub async fn init(
    settings_file: String,
    autopilot_scripts_file: String,
    config: Option<ZoomAndFocusConfig>,
    mavlink_address: String,
    mavlink_system_id: u8,
    mavlink_component_id: u8,
) -> Result<()> {
    let config = read_settings(&settings_file).unwrap_or_default();
    write_settings(&settings_file, &config)?;
    export_script(&autopilot_scripts_file, &config)?;

    let mavlink =
        MavlinkComponent::new(mavlink_address, mavlink_system_id, mavlink_component_id).await;

    let _task_handler = tokio::task::spawn(async {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    let parameters = FocusAndZoomParameters::default();

    MANAGER.get_or_init(|| {
        RwLock::new(Manager {
            config,
            parameters,
            mavlink,
            settings_file,
            autopilot_scripts_file,
            _task_handler,
        })
    });

    Ok(())
}

impl Drop for Manager {
    #[instrument(level = "debug")]
    fn drop(&mut self) {
        debug!("Finishing tasks...");

        self._task_handler.abort();
    }
}

#[instrument(level = "debug")]
pub fn export_script(path: &str, config: &ZoomAndFocusConfig) -> Result<()> {
    let contents = generate_lua_script(config)?;
    validate_lua(&contents)?;

    let path_obj = std::path::Path::new(path);
    if let Some(parent_dir) = path_obj.parent() {
        std::fs::create_dir_all(parent_dir)?;
    }

    trace!("Saving Lua script to {path:?}. Lua script content: {contents:#?}");

    std::fs::write(path_obj, contents).map_err(|error| {
        error!(?error, ?path, "Failed writing autopilot lua script");
        anyhow::Error::msg(error)
    })?;

    info!("Wrote new lua script to {path:?}");

    Ok(())
}

#[instrument(level = "debug")]
pub async fn get_config() -> ZoomAndFocusConfig {
    MANAGER.get().unwrap().read().await.config.clone()
}

#[instrument(level = "debug")]
pub async fn set_config(new_config: &ApiConfig) -> Result<ZoomAndFocusConfig> {
    let mut manager = MANAGER.get().unwrap().write().await;

    manager.update_config(new_config).await?;

    write_settings(&manager.settings_file, &manager.config)?;
    // export_script(&manager.autopilot_scripts_file, &manager.config)?; //

    Ok(manager.config.clone())
}

#[instrument(level = "debug")]
pub async fn reset() -> Result<ZoomAndFocusConfig> {
    let config = set_config(&ZoomAndFocusConfig::default().into()).await?;

    debug!("Settings resetted to default");

    Ok(config)
}

impl Manager {
    #[instrument(level = "debug")]
    pub async fn update_config(&mut self, new_config: &ApiConfig) -> Result<()> {
        if let Some(new_value) = new_config.k_focus {
            // self.mavlink
            //     .send(
            //         None,
            //         &MavMessage::PARAM_SET(PARAM_SET_DATA {
            //             target_system: self.mavlink.system_id().await,
            //             target_component: self.mavlink.component_id().await,
            //             param_id: ,
            //             param_value: (),
            //             param_type: MavParamType::MAV_PARAM_TYPE_UINT32,
            //         }),
            //     )
            //     .await;

            // TODO: wait for ACK, and only then, set the value:
            {
                info!(
                    "k_focus changed from {:?} to {new_value:?}",
                    self.config.k_focus
                );

                self.config.k_focus = new_value;
            }
        }

        // TODO: write the same pattern for all other ApiConfig fields

        Ok(())
    }

    pub async fn send_params(&mut self) -> Result<()> {
        let target_system = self.mavlink.system_id().await;
        let target_component = ::mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let encoding = self.mavlink.encoding().await.into();

        for param in self.parameters.make_params() {
            let param_id = param.param_id();
            let param_type = param.param_type();
            let param_value = param.param_value(encoding);

            self.mavlink
                .send(
                    None,
                    &MavMessage::PARAM_SET(PARAM_SET_DATA {
                        target_system,
                        target_component,
                        param_id,
                        param_value,
                        param_type,
                    }),
                )
                .await;

            // loop {
            //     let (_header, message) = self
            //         .mavlink
            //         .recv(
            //             target_system,
            //             self.mavlink.component_id().await,
            //             PARAM_VALUE_DATA::ID,
            //         )
            //         .await;

            //     let MavMessage::PARAM_VALUE(data) = message else {
            //         continue;
            //     };

            //     if data.param_id != param_id || data.param_type != param_type {
            //         continue;
            //     }

            //     if param_value != param_value {
            //         return Err(anyhow!(
            //             "Failed setting parameter {:?} to value {:?}",
            //             param.name,
            //             param.value
            //         ));
            //     }

            //     break;
            // }

            // debug!(
            //     "Parameter {:?}, sucessifully set to {:?}",
            //     param.name, param.value
            // );
        }

        Ok(())
    }
}
