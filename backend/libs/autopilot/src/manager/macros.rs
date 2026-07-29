#[macro_export]
macro_rules! generate_update_channel_param_function {
    (
        $fn_name:ident,
        $field_name:ident,
        $param_prefix:expr,
        $param_suffix:expr,
        $ty:ident,
        $channel_field:ident
    ) => {
        #[instrument(level = "debug", skip(self, parameters))]
        async fn $fn_name(
            &mut self,
            camera_uuid: &Uuid,
            parameters: &$crate::api::ActuatorsParametersConfig,
            force_apply: bool,
        ) -> Result<()> {
            let current_parameters = &mut self
                .settings
                .actuators
                .entry(*camera_uuid)
                .or_default()
                .parameters;

            let encoding = $crate::mavlink::component()?.encoding().await;

            let channel = current_parameters.$channel_field as u8;

            let param_name = format!("{}{}_{}", $param_prefix, channel, $param_suffix);

            let new_value = match (parameters.$field_name, force_apply) {
                (Some(value), _) => value,
                (None, true) => current_parameters.$field_name,
                (None, false) => return Ok(()),
            };

            let mut param = $crate::mavlink::component()?
                .get_param(&param_name, false)
                .await?;
            let old_value = current_parameters.$field_name;
            param.value.set_value(ParamType::$ty(new_value), encoding)?;

            if (old_value != new_value) || force_apply {
                match $crate::mavlink::component()?.set_param(param).await {
                    Ok(_) => {
                        if old_value != new_value {
                            info!(
                                "{} changed from {:?} to {:?}",
                                stringify!($field_name),
                                old_value,
                                new_value
                            );
                        }
                        current_parameters.$field_name = new_value;
                    }
                    Err(error) => {
                        return Err(anyhow::anyhow!(
                            "Failed setting parameter {}: {error:?}",
                            stringify!($field_name)
                        ));
                    }
                }
            } else {
                trace!("Parameter {param_name:?} skipped");
            }

            Ok(())
        }
    };
}

#[macro_export]
macro_rules! generate_update_mount_param_function {
    (
        $fn_name:ident,
        $field_name:ident,
        $param_suffix:expr,
        $ty:ident
    ) => {
        #[instrument(level = "debug", skip(self, parameters))]
        pub async fn $fn_name(
            &mut self,
            camera_uuid: &Uuid,
            parameters: &$crate::api::ActuatorsParametersConfig,
            force_apply: bool,
        ) -> Result<bool> {
            let current_parameters = &mut self
                .settings
                .actuators
                .entry(*camera_uuid)
                .or_default()
                .parameters;
            let mut has_changed = false;

            let encoding = $crate::mavlink::component()?.encoding().await;

            let mount_id = match current_parameters.camera_id {
                api::CameraID::CAM1 => TiltChannelFunction::MNT1,
                api::CameraID::CAM2 => TiltChannelFunction::MNT2,
            };
            let param_name = format!("{mount_id:?}_{}", $param_suffix);

            let new_value = match (parameters.$field_name, force_apply) {
                (Some(value), _) => value,
                (None, true) => current_parameters.$field_name,
                (None, false) => return Ok(has_changed),
            };

            let mut param = $crate::mavlink::component()?
                .get_param(&param_name, false)
                .await?;
            let old_value = current_parameters.$field_name;
            param.value.set_value(ParamType::$ty(new_value), encoding)?;

            if (old_value != new_value) || force_apply {
                match $crate::mavlink::component()?.set_param(param).await {
                    Ok(_) => {
                        if old_value != new_value {
                            info!(
                                "{} changed from {:?} to {:?}",
                                stringify!($field_name),
                                old_value,
                                new_value
                            );
                        }
                        current_parameters.$field_name = new_value;
                        has_changed = true;
                    }
                    Err(error) => {
                        return Err(anyhow::anyhow!(
                            "Failed setting parameter {}: {error:?}",
                            stringify!($field_name)
                        ));
                    }
                }
            } else {
                trace!("Parameter {param_name:?} skipped");
            }

            Ok(has_changed)
        }
    };
}
