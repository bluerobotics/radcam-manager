use anyhow::Result;
use tracing::*;
use uuid::Uuid;

use crate::{
    api, generate_update_channel_param_function, generate_update_mount_param_function,
    manager::Manager,
    parameters::{self, ParamType},
};

impl Manager {
    #[instrument(level = "debug", skip(self, parameters))]
    pub async fn update_tilt_parameters(
        &mut self,
        camera_uuid: &Uuid,
        parameters: &api::ActuatorsParametersConfig,
        overwrite: bool,
    ) -> Result<bool> {
        let mut autopilot_reboot_required = overwrite;

        if let Some(channel) = &parameters.tilt_channel {
            let current_parameters = &mut self
                .settings
                .actuators
                .entry(*camera_uuid)
                .or_default()
                .parameters;
            let encoding = self.mavlink.encoding().await;

            // Disables the old tilt_channel:
            if &current_parameters.tilt_channel != channel {
                let param_name = format!("SERVO{}_FUNCTION", current_parameters.tilt_channel as u8);

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
                                    "tilt_channel (SERVO{}) changed from {:?} to {new_value:?}",
                                    current_parameters.tilt_channel as u8, old_value
                                );
                                autopilot_reboot_required = true;
                            }
                        }
                        Err(error) => {
                            warn!(
                                "Failed to disable the old tilt channel when setting parameter: {error:?}"
                            )
                        }
                    }
                }
            }

            // Sets the new tilt_channel:
            {
                let param_name = format!("SERVO{}_FUNCTION", *channel as u8);

                let mut param = self.mavlink.get_param(&param_name, false).await?;
                let old_value = param.value;
                param.value.set_value(
                    ParamType::UINT8(parameters::TILT_CHANNEL_FUNCTION as u8),
                    encoding,
                )?;
                let new_value = param.value;

                if overwrite || old_value != new_value {
                    match self.mavlink.set_param(param).await {
                        Ok(_) => {
                            if overwrite || old_value != new_value {
                                info!(
                                    "tilt_channel (SERVO{}) changed from {:?} to {new_value:?}",
                                    *channel as u8, old_value
                                );
                            }

                            current_parameters.tilt_channel = *channel;
                            autopilot_reboot_required = true;
                        }
                        Err(error) => {
                            warn!(
                                "Failed setting new tilt channel parameter when setting parameter: {error:?}"
                            )
                        }
                    }
                }
            }
        }

        self.update_tilt_channel_parameters(camera_uuid, parameters, autopilot_reboot_required)
            .await?;

        Ok(autopilot_reboot_required)
    }
    #[instrument(level = "debug", skip(self, parameters))]
    pub async fn update_tilt_channel_parameters(
        &mut self,
        camera_uuid: &Uuid,
        parameters: &api::ActuatorsParametersConfig,
        force_apply: bool,
    ) -> Result<()> {
        self.update_tilt_channel_min(camera_uuid, parameters, force_apply)
            .await?;
        self.update_tilt_channel_trim(camera_uuid, parameters, force_apply)
            .await?;
        self.update_tilt_channel_max(camera_uuid, parameters, force_apply)
            .await?;

        self.update_tilt_mnt_pitch_min(camera_uuid, parameters, force_apply)
            .await?;
        self.update_tilt_mnt_pitch_max(camera_uuid, parameters, force_apply)
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

        let param_name = format!("{:?}_{}", parameters::TILT_CHANNEL_FUNCTION, "TYPE");

        let new_value = match (parameters.tilt_mnt_pitch_max, force_apply) {
            (Some(value), _) => value,
            (None, true) => current_parameters.tilt_mnt_pitch_max,
            (None, false) => return Ok(()),
        };
        let mut param = self.mavlink.get_param(&param_name, false).await?;
        let old_value_encoded = param.param_value(encoding)?;
        param
            .value
            .set_value(ParamType::INT32(new_value), encoding)?;
        let new_value_encoded = param.param_value(encoding)?;
        if (old_value_encoded != new_value_encoded) || force_apply {
            match self.mavlink.set_param(param).await {
                Ok(_) => {
                    if old_value_encoded != new_value_encoded {
                        info!(
                            "{} changed from {:?} to {:?}",
                            stringify!(tilt_mnt_pitch_max),
                            current_parameters.tilt_mnt_pitch_max,
                            new_value
                        );
                    }
                    current_parameters.tilt_mnt_pitch_max = new_value;

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
