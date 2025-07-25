use anyhow::Result;
use tracing::*;
use uuid::Uuid;

use crate::{
    api, generate_update_channel_param_function,
    manager::Manager,
    parameters::{self, ParamType},
};

impl Manager {
    #[instrument(level = "debug", skip(self, parameters))]
    pub async fn update_focus_parameters(
        &mut self,
        camera_uuid: &Uuid,
        parameters: &api::ActuatorsParametersConfig,
        overwrite: bool,
    ) -> Result<bool> {
        let mut autopilot_reboot_required = overwrite;

        if let Some(channel) = &parameters.focus_channel {
            let current_parameters = &mut self
                .settings
                .actuators
                .entry(*camera_uuid)
                .or_default()
                .parameters;
            let encoding = self.mavlink.encoding().await;

            // Disables the old focus_channel:
            if &current_parameters.focus_channel != channel {
                let param_name =
                    format!("SERVO{}_FUNCTION", current_parameters.focus_channel as u8);

                let mut param = self.mavlink.get_param(&param_name, false).await?;
                let old_value = param.value;
                param.value.set_value(
                    ParamType::UINT8(parameters::DISABLED_CHANNEL_FUNCTION),
                    encoding,
                )?;
                let new_value = param.value;

                if old_value != new_value {
                    match self.mavlink.set_param(param).await {
                        Ok(_) => {
                            if old_value != new_value {
                                info!(
                                    "focus_channel (SERVO{}) changed from {:?} to {new_value:?}",
                                    current_parameters.focus_channel as u8, old_value
                                );
                                autopilot_reboot_required = true;
                            }
                        }
                        Err(error) => {
                            warn!(
                                "Failed to disable the old focus channel when setting parameter: {error:?}"
                            )
                        }
                    }
                }
            }

            // Sets the new focus_channel:
            {
                let param_name = format!("SERVO{}_FUNCTION", *channel as u8);

                let mut param = self.mavlink.get_param(&param_name, false).await?;
                let old_value = param.value;
                param.value.set_value(
                    ParamType::UINT8(parameters::FOCUS_CHANNEL_FUNCTION),
                    encoding,
                )?;
                let new_value = param.value;

                if overwrite || old_value != new_value {
                    match self.mavlink.set_param(param).await {
                        Ok(_) => {
                            if overwrite || old_value != new_value {
                                info!(
                                    "focus_channel (SERVO{}) changed from {:?} to {new_value:?}",
                                    *channel as u8, old_value
                                );
                            }

                            current_parameters.focus_channel = *channel;
                            autopilot_reboot_required = true;
                        }
                        Err(error) => {
                            warn!(
                                "Failed setting new focus channel parameter when setting parameter: {error:?}"
                            )
                        }
                    }
                }
            }
        }

        self.update_focus_channel_parameters(camera_uuid, parameters, autopilot_reboot_required)
            .await?;

        Ok(autopilot_reboot_required)
    }

    #[instrument(level = "debug", skip(self, parameters))]
    async fn update_focus_channel_parameters(
        &mut self,
        camera_uuid: &Uuid,
        parameters: &api::ActuatorsParametersConfig,
        force_apply: bool,
    ) -> Result<()> {
        self.update_focus_channel_min(camera_uuid, parameters, force_apply)
            .await?;
        self.update_focus_channel_trim(camera_uuid, parameters, force_apply)
            .await?;
        self.update_focus_channel_max(camera_uuid, parameters, force_apply)
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
