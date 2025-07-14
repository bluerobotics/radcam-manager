mod defaults;
mod mavlink;
pub mod parameters;
pub mod routes;
mod settings;

pub use routes::{ZoomAndFocusConfigQuery, router};

use anyhow::Result;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::*;
use ts_rs::TS;

use crate::{
    mavlink::MavlinkComponent,
    parameters::{
        FocusAndZoomParameters, FocusAndZoomParametersQuery, ParamType, TILT_CHANNEL_FUNCTION,
        TiltChannelFunction,
    },
    settings::{read_settings, write_settings},
};

static MANAGER: OnceCell<RwLock<Manager>> = OnceCell::new();

#[derive(Debug)]
struct Manager {
    config: ZoomAndFocusConfig,
    mavlink: MavlinkComponent,
    settings_file: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZoomAndFocusConfig {
    pub parameters: FocusAndZoomParameters,
    pub closest_points: FocusZoomPoints,
    pub furthest_points: FocusZoomPoints,
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

/// Constructs our manager, Should be done inside main
#[instrument(level = "debug")]
pub async fn init(
    settings_file: String,
    config: Option<ZoomAndFocusConfig>,
    mavlink_address: String,
    mavlink_system_id: u8,
    mavlink_component_id: u8,
) -> Result<()> {
    let config = read_settings(&settings_file).unwrap_or_default();
    write_settings(&settings_file, &config)?;

    let mavlink =
        MavlinkComponent::new(mavlink_address, mavlink_system_id, mavlink_component_id).await;

    tokio::spawn(async {
        // DEVELOPMENT ONLY!
        // DEVELOPMENT ONLY!
        // DEVELOPMENT ONLY!
        // DEVELOPMENT ONLY!
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            let mut manager = MANAGER.get().unwrap().write().await;

            debug!("SETTING focus_channel TO SERVO10!");

            manager
                .update_config(&ZoomAndFocusConfigQuery {
                    parameters: Some(FocusAndZoomParametersQuery {
                        focus_channel: Some(parameters::ServoChannel::SERVO10),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await
                .unwrap();

            debug!("YAYYYYYYYYY!");
        }
    });

    MANAGER.get_or_init(|| {
        RwLock::new(Manager {
            config,
            mavlink,
            settings_file,
        })
    });

    Ok(())
}

#[instrument(level = "debug")]
pub async fn get_config() -> ZoomAndFocusConfig {
    MANAGER.get().unwrap().read().await.config.clone()
}

#[instrument(level = "debug")]
pub async fn set_config(new_config: &ZoomAndFocusConfigQuery) -> Result<ZoomAndFocusConfig> {
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

macro_rules! generate_update_channel_param_function {
    (
        $fn_name:ident,
        $field_name:ident,
        $param_prefix:expr,
        $param_suffix:expr,
        $ty:ident,
        $channel_field:ident
    ) => {
        #[instrument(level = "debug", skip(self))]
        async fn $fn_name(
            &mut self,
            parameters: &FocusAndZoomParametersQuery,
            force_apply: bool,
        ) -> Result<()> {
            let encoding = self.mavlink.encoding().await;

            let channel = self.config.parameters.$channel_field as u8;

            let param_name = format!("{}{}_{}", $param_prefix, channel, $param_suffix);

            let new_value = match (parameters.$field_name, force_apply) {
                (Some(value), _) => value,
                (None, true) => self.config.parameters.$field_name,
                (None, false) => return Ok(()),
            };

            let mut param = self.mavlink.get_param(&param_name, false).await?;
            let old_value = param.param_value(encoding)?;
            param.value.set_value(ParamType::$ty(new_value), encoding)?;

            if (old_value != param.param_value(encoding)?) || force_apply {
                match self.mavlink.set_param(param).await {
                    Ok(_) => {
                        info!(
                            "{} changed from {:?} to {:?}",
                            stringify!($field_name),
                            self.config.parameters.$field_name,
                            new_value
                        );
                        self.config.parameters.$field_name = new_value;
                    }
                    Err(error) => {
                        warn!("Failed setting parameter {param_name:?}: {error:?}")
                    }
                }
            } else {
                trace!("Parameter {param_name:?} skipped");
            }

            Ok(())
        }
    };
}

macro_rules! generate_update_mount_param_function {
    (
        $fn_name:ident,
        $field_name:ident,
        $param_suffix:expr,
        $ty:ident
    ) => {
        #[instrument(level = "debug", skip(self))]
        pub async fn $fn_name(
            &mut self,
            parameters: &FocusAndZoomParametersQuery,
            force_apply: bool,
        ) -> Result<()> {
            let encoding = self.mavlink.encoding().await;

            let param_name = format!("{:?}_{}", TILT_CHANNEL_FUNCTION, "PITCH_MAX");

            let new_value = match (parameters.$field_name, force_apply) {
                (Some(value), _) => value,
                (None, true) => self.config.parameters.$field_name,
                (None, false) => return Ok(()),
            };

            let mut param = self.mavlink.get_param(&param_name, false).await?;
            let old_value = param.param_value(encoding)?;
            param.value.set_value(ParamType::$ty(new_value), encoding)?;

            if (old_value != param.param_value(encoding)?) || force_apply {
                match self.mavlink.set_param(param).await {
                    Ok(_) => {
                        info!(
                            "{} changed from {:?} to {:?}",
                            stringify!($field_name),
                            self.config.parameters.$field_name,
                            new_value
                        );
                        self.config.parameters.$field_name = new_value;
                    }
                    Err(error) => {
                        warn!("Failed setting parameter: {error:?}")
                    }
                }
            }

            Ok(())
        }
    };
}

impl Manager {
    #[instrument(level = "debug", skip(self))]
    pub async fn update_config(&mut self, new_config: &ZoomAndFocusConfigQuery) -> Result<()> {
        // Parameters update
        if let Some(parameters) = &new_config.parameters {
            self.update_focus_parameters(parameters).await?;

            self.update_zoom_parameters(parameters).await?;

            self.update_tilt_parameters(parameters).await?;
        }

        // Callibration update
        if let Some(points) = &new_config.closest_points {
            self.update_closest_points(points).await?;
        }
        if let Some(points) = &new_config.furthest_points {
            self.update_furthest_points(points).await?;
        }

        Ok(())
    }
}

// FOCUS part
impl Manager {
    #[instrument(level = "debug", skip(self))]
    async fn update_focus_parameters(
        &mut self,
        parameters: &FocusAndZoomParametersQuery,
    ) -> Result<()> {
        if let Some(new_value) = &parameters.focus_channel {
            let encoding = self.mavlink.encoding().await;

            let param_name = format!(
                "SERVO{}_FUNCTION",
                self.config.parameters.focus_channel as u8
            );

            let mut param = self.mavlink.get_param(&param_name, false).await?;
            let old_value = param.param_value(encoding)?;
            param
                .value
                .set_value(ParamType::UINT8(*new_value as u8), encoding)?;

            if old_value != param.param_value(encoding)? {
                match self.mavlink.set_param(param).await {
                    Ok(_) => {
                        info!(
                            "focus_channel changed from {:?} to {new_value:?}",
                            self.config.parameters.focus_channel
                        );

                        self.config.parameters.focus_channel = *new_value;
                    }
                    Err(error) => {
                        warn!("Failed setting parameter: {error:?}")
                    }
                }
            }
        }

        self.update_focus_channel_parameters(parameters, parameters.focus_channel.is_some())
            .await
    }

    #[instrument(level = "debug", skip(self))]
    async fn update_focus_channel_parameters(
        &mut self,
        parameters: &FocusAndZoomParametersQuery,
        force_apply: bool,
    ) -> Result<()> {
        // self.update_focus_channel(parameters, force_apply).await?;
        self.update_focus_channel_min(parameters, force_apply)
            .await?;
        self.update_focus_channel_trim(parameters, force_apply)
            .await?;
        self.update_focus_channel_max(parameters, force_apply)
            .await?;

        Ok(())
    }

    generate_update_channel_param_function!(
        update_focus_channel_min,
        focus_channel_min,
        "SERVO",
        "MIN",
        UINT16,
        focus_channel
    );

    generate_update_channel_param_function!(
        update_focus_channel_max,
        focus_channel_max,
        "SERVO",
        "MAX",
        UINT16,
        focus_channel
    );

    generate_update_channel_param_function!(
        update_focus_channel_trim,
        focus_channel_trim,
        "SERVO",
        "TRIM",
        UINT16,
        focus_channel
    );
}

// ZOOM part
impl Manager {
    #[instrument(level = "debug", skip(self))]
    async fn update_zoom_parameters(
        &mut self,
        parameters: &FocusAndZoomParametersQuery,
    ) -> Result<()> {
        if let Some(new_value) = &parameters.zoom_channel {
            let encoding = self.mavlink.encoding().await;

            let param_name = format!(
                "SERVO{}_FUNCTION",
                self.config.parameters.zoom_channel as u8
            );

            let mut param = self.mavlink.get_param(&param_name, false).await?;
            let old_value = param.param_value(encoding)?;
            param
                .value
                .set_value(ParamType::UINT8(*new_value as u8), encoding)?;

            if old_value != param.param_value(encoding)? {
                match self.mavlink.set_param(param).await {
                    Ok(_) => {
                        info!(
                            "zoom_channel changed from {:?} to {new_value:?}",
                            self.config.parameters.zoom_channel
                        );

                        self.config.parameters.zoom_channel = *new_value;
                    }
                    Err(error) => {
                        warn!("Failed setting parameter: {error:?}")
                    }
                }
            }
        }

        self.update_zoom_channel_parameters(parameters, parameters.zoom_channel.is_some())
            .await
    }

    #[instrument(level = "debug", skip(self))]
    async fn update_zoom_channel_parameters(
        &mut self,
        parameters: &FocusAndZoomParametersQuery,
        force_apply: bool,
    ) -> Result<()> {
        self.update_zoom_channel_min(parameters, force_apply)
            .await?;
        self.update_zoom_channel_trim(parameters, force_apply)
            .await?;
        self.update_zoom_channel_max(parameters, force_apply)
            .await?;

        Ok(())
    }

    generate_update_channel_param_function!(
        update_zoom_channel_min,
        zoom_channel_min,
        "SERVO",
        "MIN",
        UINT16,
        zoom_channel
    );

    generate_update_channel_param_function!(
        update_zoom_channel_max,
        zoom_channel_max,
        "SERVO",
        "MAX",
        UINT16,
        zoom_channel
    );

    generate_update_channel_param_function!(
        update_zoom_channel_trim,
        zoom_channel_trim,
        "SERVO",
        "TRIM",
        UINT16,
        zoom_channel
    );
}

// TILT part
impl Manager {
    #[instrument(level = "debug", skip(self))]
    async fn update_tilt_parameters(
        &mut self,
        parameters: &FocusAndZoomParametersQuery,
    ) -> Result<()> {
        if let Some(new_value) = &parameters.tilt_channel {
            let encoding = self.mavlink.encoding().await;

            let param_name = format!(
                "SERVO{}_FUNCTION",
                self.config.parameters.tilt_channel as u8
            );

            let mut param = self.mavlink.get_param(&param_name, false).await?;
            let old_value = param.param_value(encoding)?;
            param
                .value
                .set_value(ParamType::UINT8(*new_value as u8), encoding)?;

            if old_value != param.param_value(encoding)? {
                match self.mavlink.set_param(param).await {
                    Ok(_) => {
                        info!(
                            "tilt_channel changed from {:?} to {new_value:?}",
                            self.config.parameters.tilt_channel
                        );

                        self.config.parameters.tilt_channel = *new_value;
                    }
                    Err(error) => {
                        warn!("Failed setting parameter: {error:?}")
                    }
                }
            }
        }

        self.update_tilt_channel_parameters(parameters, parameters.tilt_channel.is_some())
            .await
    }
    #[instrument(level = "debug", skip(self))]
    async fn update_tilt_channel_parameters(
        &mut self,
        parameters: &FocusAndZoomParametersQuery,
        force_apply: bool,
    ) -> Result<()> {
        self.update_tilt_channel_min(parameters, force_apply)
            .await?;
        self.update_tilt_channel_trim(parameters, force_apply)
            .await?;
        self.update_tilt_channel_max(parameters, force_apply)
            .await?;

        self.update_tilt_mnt_pitch_min(parameters, force_apply)
            .await?;
        self.update_tilt_mnt_pitch_max(parameters, force_apply)
            .await?;

        Ok(())
    }

    generate_update_channel_param_function!(
        update_tilt_channel_min,
        tilt_channel_min,
        "SERVO",
        "MIN",
        UINT16,
        tilt_channel
    );

    generate_update_channel_param_function!(
        update_tilt_channel_max,
        tilt_channel_max,
        "SERVO",
        "MAX",
        UINT16,
        tilt_channel
    );

    generate_update_channel_param_function!(
        update_tilt_channel_trim,
        tilt_channel_trim,
        "SERVO",
        "TRIM",
        UINT16,
        tilt_channel
    );

    generate_update_mount_param_function!(
        update_tilt_mnt_pitch_min,
        tilt_mnt_pitch_min,
        "PITCH_MIN",
        INT32
    );

    generate_update_mount_param_function!(
        update_tilt_mnt_pitch_max,
        tilt_mnt_pitch_max,
        "PITCH_MAX",
        INT32
    );

    #[instrument(level = "debug", skip(self))]
    pub async fn update_tilt_mnt_type(
        &mut self,
        parameters: &FocusAndZoomParametersQuery,
        force_apply: bool,
    ) -> Result<()> {
        let encoding = self.mavlink.encoding().await;

        let param_name = format!("{:?}_{}", TILT_CHANNEL_FUNCTION, "TYPE");

        let new_value = match (parameters.tilt_mnt_pitch_max, force_apply) {
            (Some(value), _) => value,
            (None, true) => self.config.parameters.tilt_mnt_pitch_max,
            (None, false) => return Ok(()),
        };
        let mut param = self.mavlink.get_param(&param_name, false).await?;
        let old_value = param.param_value(encoding)?;
        param
            .value
            .set_value(ParamType::INT32(new_value), encoding)?;
        if (old_value != param.param_value(encoding)?) || force_apply {
            match self.mavlink.set_param(param).await {
                Ok(_) => {
                    info!(
                        "{} changed from {:?} to {:?}",
                        stringify!(tilt_mnt_pitch_max),
                        self.config.parameters.tilt_mnt_pitch_max,
                        new_value
                    );
                    self.config.parameters.tilt_mnt_pitch_max = new_value;

                    // TODO: Reboot required after change!
                }
                Err(error) => {
                    warn!("Failed setting parameter: {error:?}")
                }
            }
        }

        Ok(())
    }
}

// Callibration part
impl Manager {
    #[instrument(level = "debug", skip(self))]
    async fn update_closest_points(&mut self, points: &FocusZoomPoints) -> Result<()> {
        todo!()
    }

    #[instrument(level = "debug", skip(self))]
    async fn update_furthest_points(&mut self, points: &FocusZoomPoints) -> Result<()> {
        todo!()
    }
}
