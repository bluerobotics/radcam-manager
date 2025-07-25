use anyhow::Result;
use tracing::*;
use uuid::Uuid;

use crate::{
    api,
    manager::Manager,
    parameters::{self, ParamType},
};

impl Manager {
    #[instrument(level = "debug", skip(self, parameters))]
    pub async fn update_camera_parameters(
        &mut self,
        camera_uuid: &Uuid,
        parameters: &api::ActuatorsParametersConfig,
        overwrite: bool,
    ) -> Result<bool> {
        let mut autopilot_reboot_required = overwrite;

        if let Some(camera_id) = &parameters.camera_id {
            let current_parameters = &mut self
                .settings
                .actuators
                .entry(*camera_uuid)
                .or_default()
                .parameters;
            let encoding = self.mavlink.encoding().await;

            // Disables the old camera_id:
            if &current_parameters.camera_id != camera_id {
                let param_name = format!("CAM{}_TYPE", current_parameters.camera_id as u8);

                let mut param = self.mavlink.get_param(&param_name, false).await?;
                let old_value = param.value;
                param
                    .value
                    .set_value(ParamType::UINT8(parameters::DISABLED_CAMERA_TYPE), encoding)?;
                let new_value = param.value;

                if old_value != new_value {
                    match self.mavlink.set_param(param).await {
                        Ok(_) => {
                            if old_value != new_value {
                                info!(
                                    "camera_id (CAM{}) changed from {:?} to {new_value:?}",
                                    current_parameters.camera_id as u8, old_value
                                );
                                autopilot_reboot_required = true;
                            }
                        }
                        Err(error) => {
                            warn!(
                                "Failed to disable the old camera camera_id when setting parameter: {error:?}"
                            )
                        }
                    }
                }
            }

            // Sets the new camera_id:
            {
                let param_name = format!("CAM{}_TYPE", *camera_id as u8);

                let mut param = self.mavlink.get_param(&param_name, false).await?;
                let old_value = param.value;
                param
                    .value
                    .set_value(ParamType::UINT8(parameters::SERVO_CAMERA_TYPE), encoding)?;
                let new_value = param.value;

                if overwrite || old_value != new_value {
                    match self.mavlink.set_param(param).await {
                        Ok(_) => {
                            if overwrite || old_value != new_value {
                                info!(
                                    "camera_id (CAM{}) changed from {:?} to {new_value:?}",
                                    *camera_id as u8, old_value
                                );
                            }

                            current_parameters.camera_id = *camera_id;
                            autopilot_reboot_required = true;
                        }
                        Err(error) => {
                            warn!(
                                "Failed setting new camera camera_id parameter when setting parameter: {error:?}"
                            )
                        }
                    }
                }
            }
        }

        Ok(autopilot_reboot_required)
    }
}
