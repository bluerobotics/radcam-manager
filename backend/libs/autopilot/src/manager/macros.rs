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
            parameters: &crate::api::ActuatorsParametersConfig,
            force_apply: bool,
        ) -> Result<()> {
            let current_parameters = &mut self
                .actuators
                .entry(*camera_uuid)
                .or_insert_with(|| CameraActuators {
                    camera_uuid: *camera_uuid,
                    config: Config::default(),
                    state: crate::api::ActuatorsState::default(),
                })
                .config
                .parameters;

            let encoding = self.mavlink.encoding().await;

            let channel = current_parameters.$channel_field as u8;

            let param_name = format!("{}{}_{}", $param_prefix, channel, $param_suffix);

            let new_value = match (parameters.$field_name, force_apply) {
                (Some(value), _) => value,
                (None, true) => current_parameters.$field_name,
                (None, false) => return Ok(()),
            };

            let mut param = self.mavlink.get_param(&param_name, false).await?;
            let old_value = current_parameters.$field_name;
            param.value.set_value(ParamType::$ty(new_value), encoding)?;

            if (old_value != new_value) || force_apply {
                match self.mavlink.set_param(param).await {
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
                        warn!("Failed setting parameter: {error:?}")
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
            parameters: &crate::api::ActuatorsParametersConfig,
            force_apply: bool,
        ) -> Result<bool> {
            let current_parameters = &mut self
                .actuators
                .entry(*camera_uuid)
                .or_insert_with(|| CameraActuators {
                    camera_uuid: *camera_uuid,
                    config: Config::default(),
                    state: crate::api::ActuatorsState::default(),
                })
                .config
                .parameters;
            let mut has_changed = false;

            let encoding = self.mavlink.encoding().await;

            let param_name = format!("{:?}_{}", parameters::TILT_CHANNEL_FUNCTION, "PITCH_MAX");

            let new_value = match (parameters.$field_name, force_apply) {
                (Some(value), _) => value,
                (None, true) => current_parameters.$field_name,
                (None, false) => return Ok(has_changed),
            };

            let mut param = self.mavlink.get_param(&param_name, false).await?;
            let old_value = current_parameters.$field_name;
            param.value.set_value(ParamType::$ty(new_value), encoding)?;

            if (old_value != new_value) || force_apply {
                match self.mavlink.set_param(param).await {
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
                        warn!("Failed setting parameter: {error:?}")
                    }
                }
            } else {
                trace!("Parameter {param_name:?} skipped");
            }

            Ok(has_changed)
        }
    };
}
