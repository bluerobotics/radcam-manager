use anyhow::{Context, Result};
use mlua::Lua;
use tera::Tera;
use tracing::*;
use uuid::Uuid;

use crate::{
    CameraActuators, api, generate_update_channel_param_function,
    manager::Manager,
    parameters::{ChannelFunction, ParamType},
};

const PARAM_TABLE_KEY_BASE: u8 = 73;
pub const PARAM_PREFIX: &str = "RCAM";

impl Manager {
    #[instrument(level = "debug", skip(self))]
    pub async fn export_script(&mut self, camera_uuid: &Uuid, overwrite: bool) -> Result<bool> {
        let camera_actuators = self
            .settings
            .actuators
            .get(camera_uuid)
            .context("Camera's actuators not configured")?;
        let path = &self.autopilot_scripts_file;

        let contents = generate_lua_script(camera_actuators)?;

        validate_lua(&contents)?;

        let path_obj = std::path::Path::new(path);
        if let Some(parent_dir) = path_obj.parent() {
            tokio::fs::create_dir_all(parent_dir).await?;
        }

        if let Ok(existing_contents) = tokio::fs::read_to_string(path_obj).await
            && !overwrite
            && existing_contents == contents
        {
            return Ok(false);
        }

        trace!("Saving Lua script to {path:?}. Lua script content: {contents:#?}");

        tokio::fs::write(path_obj, contents)
            .await
            .map_err(|error| {
                error!(?error, ?path, "Failed writing autopilot lua script");
                anyhow::Error::msg(error)
            })?;

        info!("Wrote new lua script to {path:?}");

        self.settings.save().await?;

        Ok(true)
    }

    #[instrument(level = "debug", skip(self, parameters))]
    pub async fn update_script_parameters(
        &mut self,
        camera_uuid: &Uuid,
        parameters: &api::ActuatorsParametersConfig,
        overwrite: bool,
    ) -> Result<bool> {
        let mut autopilot_reboot_required = overwrite;

        if let Some(channel) = &parameters.script_channel {
            let current_parameters = &mut self
                .settings
                .actuators
                .entry(*camera_uuid)
                .or_default()
                .parameters;
            let encoding = self.mavlink.encoding().await;

            // Disables the old script_channel:
            if &current_parameters.script_channel != channel {
                let param_name =
                    format!("SERVO{}_FUNCTION", current_parameters.script_channel as u8);

                let mut param = self.mavlink.get_param(&param_name, false).await?;
                let old_value = param.value;
                param
                    .value
                    .set_value(ParamType::INT16(ChannelFunction::Disabled as i16), encoding)?;
                let new_value = param.value;

                if old_value != new_value {
                    match self.mavlink.set_param(param).await {
                        Ok(_) => {
                            if old_value != new_value {
                                info!(
                                    "script_channel (SERVO{}) changed from {old_value:?} to {new_value:?}",
                                    current_parameters.script_channel as u8,
                                );
                                autopilot_reboot_required = true;
                            }
                        }
                        Err(error) => {
                            warn!(
                                "Failed to disable the old script channel when setting parameter: {error:?}"
                            )
                        }
                    }
                }
            }

            // Sets the new script_channel:
            {
                let param_name = format!("SERVO{}_FUNCTION", *channel as u8);

                // The script servo input is the values from the CameraFocus
                let function = ChannelFunction::CameraFocus;

                let mut param = self.mavlink.get_param(&param_name, false).await?;
                let old_value = param.value;
                param
                    .value
                    .set_value(ParamType::INT16(function as i16), encoding)?;
                let new_value = param.value;

                if overwrite || old_value != new_value {
                    match self.mavlink.set_param(param).await {
                        Ok(_) => {
                            if overwrite || old_value != new_value {
                                info!(
                                    "script_channel (SERVO{}) changed from {old_value:?} to {new_value:?}",
                                    *channel as u8
                                );
                            }

                            current_parameters.script_channel = *channel;
                            autopilot_reboot_required = true;
                        }
                        Err(error) => {
                            warn!(
                                "Failed setting new script channel parameter when setting parameter: {error:?}"
                            )
                        }
                    }
                }
            }
        }

        self.update_script_channel_parameters(camera_uuid, parameters, autopilot_reboot_required)
            .await?;

        Ok(autopilot_reboot_required)
    }

    pub async fn update_script_enable(
        &mut self,
        camera_uuid: &Uuid,
        parameters: &api::ActuatorsParametersConfig,
        force_apply: bool,
    ) -> Result<()> {
        let current_parameters = &mut self
            .settings
            .actuators
            .entry(*camera_uuid)
            .or_default()
            .parameters;

        let encoding = self.mavlink.encoding().await;

        let channel = current_parameters.camera_id as u8;

        let param_name = format!("{PARAM_PREFIX}{channel}_ENABLE");

        let new_value = match (parameters.enable_focus_and_zoom_correlation, force_apply) {
            (Some(value), _) => value,
            (None, true) => current_parameters.enable_focus_and_zoom_correlation,
            (None, false) => return Ok(()),
        };

        let mut param = self.mavlink.get_param(&param_name, false).await?;
        let old_value = current_parameters.enable_focus_and_zoom_correlation;
        param
            .value
            .set_value(ParamType::UINT8(new_value as u8), encoding)?;

        if (old_value != new_value) || force_apply {
            match self.mavlink.set_param(param).await {
                Ok(_) => {
                    if old_value != new_value {
                        info!(
                            "{} changed from {old_value:?} to {new_value:?}",
                            stringify!(enable_focus_and_zoom_correlation),
                        );
                    }
                    current_parameters.enable_focus_and_zoom_correlation = new_value;
                }
                Err(error) => {
                    warn!("Failed setting parameter: {error:?}")
                }
            }
        } else {
            trace!("Parameter {param_name:?} skipped");
        }

        Ok(())
    }

    pub async fn update_script_gain(
        &mut self,
        camera_uuid: &Uuid,
        parameters: &api::ActuatorsParametersConfig,
        force_apply: bool,
    ) -> Result<()> {
        let current_parameters = &mut self
            .settings
            .actuators
            .entry(*camera_uuid)
            .or_default()
            .parameters;

        let encoding = self.mavlink.encoding().await;

        let channel = current_parameters.camera_id as u8;

        let param_name = format!("{PARAM_PREFIX}{channel}_GAIN");

        let new_value = match (parameters.focus_margin_gain, force_apply) {
            (Some(value), _) => value,
            (None, true) => current_parameters.focus_margin_gain,
            (None, false) => return Ok(()),
        };

        let mut param = self.mavlink.get_param(&param_name, false).await?;
        let old_value = current_parameters.focus_margin_gain;
        param
            .value
            .set_value(ParamType::UINT8(new_value as u8), encoding)?;

        if (old_value != new_value) || force_apply {
            match self.mavlink.set_param(param).await {
                Ok(_) => {
                    if old_value != new_value {
                        info!(
                            "{} changed from {old_value:?} to {new_value:?}",
                            stringify!(focus_margin_gain),
                        );
                    }
                    current_parameters.focus_margin_gain = new_value;
                }
                Err(error) => {
                    warn!("Failed setting parameter: {error:?}")
                }
            }
        } else {
            trace!("Parameter {param_name:?} skipped");
        }

        Ok(())
    }

    #[instrument(level = "debug", skip(self, parameters))]
    pub async fn update_script_channel_parameters(
        &mut self,
        camera_uuid: &Uuid,
        parameters: &api::ActuatorsParametersConfig,
        force_apply: bool,
    ) -> Result<()> {
        self.update_script_channel_min(camera_uuid, parameters, force_apply)
            .await?;
        self.update_script_channel_trim(camera_uuid, parameters, force_apply)
            .await?;
        self.update_script_channel_max(camera_uuid, parameters, force_apply)
            .await?;

        Ok(())
    }

    generate_update_channel_param_function!(
        update_script_channel_min,
        script_channel_min,
        "SERVO",
        "MIN",
        UINT16,
        script_channel
    );

    generate_update_channel_param_function!(
        update_script_channel_max,
        script_channel_max,
        "SERVO",
        "MAX",
        UINT16,
        script_channel
    );

    generate_update_channel_param_function!(
        update_script_channel_trim,
        script_channel_trim,
        "SERVO",
        "TRIM",
        UINT16,
        script_channel
    );
}

fn generate_lua_script(config: &CameraActuators) -> Result<String> {
    let mut context = tera::Context::new();

    let channel = config.parameters.camera_id as u8;

    let param_table_key = PARAM_TABLE_KEY_BASE + channel;
    let param_prefix = format!("\"{PARAM_PREFIX}{channel}_\"");

    context.insert("param_table_key", &param_table_key);
    context.insert("param_prefix", &param_prefix);
    context.insert("margin_gain", &{ config.parameters.focus_margin_gain });
    context.insert("k_script", &(config.parameters.script_function as u8));
    context.insert("closest_points", &config.closest_points.to_lua());
    context.insert("furthest_points", &config.furthest_points.to_lua());
    context.insert("version", env!("CARGO_PKG_VERSION"));

    let template = include_str!("radcam.lua.template");

    let file = Tera::one_off(template, &context, false)?;

    Ok(file)
}

fn validate_lua(script: &str) -> Result<()> {
    Lua::new()
        .load(script)
        .set_mode(mlua::ChunkMode::Text)
        .into_function()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_script_generation() {
        let contents = generate_lua_script(&CameraActuators::default()).unwrap();
        dbg!(&contents);

        validate_lua(&contents).unwrap();
    }
}
