use anyhow::Result;
use tracing::*;
use uuid::Uuid;

use crate::{
    CameraActuators, Config, api, generate_update_channel_param_function,
    manager::Manager,
    parameters::{self, ParamType},
};
impl Manager {
    #[instrument(level = "debug", skip(self, parameters))]
    pub async fn update_script_parameters(
        &mut self,
        camera_uuid: &Uuid,
        parameters: &api::ActuatorsParametersConfig,
    ) -> Result<bool> {
        let mut autopilot_reboot_required = false;

        if let Some(channel) = &parameters.script_channel {
            let current_parameters = &mut self
                .actuators
                .entry(*camera_uuid)
                .or_insert_with(|| CameraActuators {
                    camera_uuid: *camera_uuid,
                    config: Config::default(),
                    state: api::ActuatorsState::default(),
                })
                .config
                .parameters;
            let encoding = self.mavlink.encoding().await;

            // Disables the old script_channel:
            if &current_parameters.script_channel != channel {
                let param_name =
                    format!("SERVO{}_FUNCTION", current_parameters.script_channel as u8);

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
                                    "script_channel (SERVO{}) changed from {:?} to {new_value:?}",
                                    *channel as u8, old_value
                                );
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

                let mut param = self.mavlink.get_param(&param_name, false).await?;
                let old_value = param.value;
                param.value.set_value(
                    ParamType::UINT8(parameters::SCRIPT_CHANNEL_FUNCTION),
                    encoding,
                )?;
                let new_value = param.value;

                if old_value != new_value {
                    match self.mavlink.set_param(param).await {
                        Ok(_) => {
                            if old_value != new_value {
                                info!(
                                    "script_channel (SERVO{}) changed from {:?} to {new_value:?}",
                                    *channel as u8, old_value
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

        self.update_script_channel_parameters(
            camera_uuid,
            parameters,
            parameters.script_channel.is_some(),
        )
        .await?;

        Ok(autopilot_reboot_required)
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

#[instrument(level = "debug")]
pub async fn export_script(path: &str) -> Result<()> {
    let contents = generate_lua_script()?;

    let path_obj = std::path::Path::new(path);
    if let Some(parent_dir) = path_obj.parent() {
        tokio::fs::create_dir_all(parent_dir).await?;
    }

    trace!("Saving Lua script to {path:?}. Lua script content: {contents:#?}");

    tokio::fs::write(path_obj, contents)
        .await
        .map_err(|error| {
            error!(?error, ?path, "Failed writing autopilot lua script");
            anyhow::Error::msg(error)
        })?;

    info!("Wrote new lua script to {path:?}");

    Ok(())
}

pub fn generate_lua_script() -> Result<String> {
    let file = r#"
--- Focus correction script.

-- Usage: change the output controlling focus from CameraFocus to "Script1"
--        but assign CameraFocus to any other "Disabled" channel as we need to read it, as it allows fine-tuning

K_FOCUS = 92
K_ZOOM = 180
K_SCRIPTING1 = 94

local MARGIN_GAIN = 1.05   -- this will allow us to move 5% beyeond closest/furthest points

-- Lookup tables for closest and furthest focus points
local closest_points = {
    {zoom = 900,  focus = 882},
    {zoom = 1100, focus = 1253},
    {zoom = 1300, focus = 1498},
    {zoom = 1500, focus = 1669},
    {zoom = 1700, focus = 1759},
    {zoom = 1900, focus = 1862},
    {zoom = 2100, focus = 1883}
}

local furthest_points = {
    {zoom = 900,  focus = 935},
    {zoom = 1100, focus = 1305},
    {zoom = 1300, focus = 1520},
    {zoom = 1500, focus = 1696},
    {zoom = 1700, focus = 1811},
    {zoom = 1900, focus = 1911},
    {zoom = 2100, focus = 1930}
}

-- local focus_channel = SRV_Channels:find_channel(K_FOCUS)
-- local zoom_channel = SRV_Channels:find_channel(K_ZOOM)

-- -- set zoom to trim levels
-- SRV_Channels:set_output_pwm(zoom_channel, 1000)
-- SRV_Channels:set_range(zoom_channel, 1000)
-- SRV_Channels:set_output_scaled(zoom_channel, 0)

-- Linear interpolation function
local function lerp(x, x1, y1, x2, y2)
    return y1 + (x - x1) * (y2 - y1) / (x2 - x1)
end

-- Function to interpolate focus value from lookup table
local function interpolate_focus(zoom, points)
    -- Handle edge cases
    if zoom <= points[1].zoom then
        return points[1].focus
    end
    if zoom >= points[#points].zoom then
        return points[#points].focus
    end
    
    -- Find the bracketing points
    for i = 1, #points - 1 do
        if zoom >= points[i].zoom and zoom < points[i + 1].zoom then
            return lerp(zoom, 
                       points[i].zoom, points[i].focus,
                       points[i + 1].zoom, points[i + 1].focus)
        end
    end
    
    return points[#points].focus -- fallback
end

-- Function to calculate focus position based on zoom position
local function calculate_focus()
    local focus = SRV_Channels:get_output_pwm(K_FOCUS)
    local focus_delta = 0.5 + MARGIN_GAIN * (focus - 1500) / 400.0 -- focus_delta is [0,1], assuming default 1100-1900 limits
    local zoom = SRV_Channels:get_output_pwm(K_ZOOM)
    -- Interpolate both closest and furthest focus values
    local closest_focus = interpolate_focus(zoom, closest_points)
    local furthest_focus = interpolate_focus(zoom, furthest_points)
    
    -- Linear interpolation between closest and furthest based on focus_delta
    return math.floor(closest_focus + focus_delta * (furthest_focus - closest_focus))
end

function update()
    local focus_pos = calculate_focus()
    SRV_Channels:set_output_pwm(K_SCRIPTING1, focus_pos)
    return update, 100
end

return update, 100
    "#;

    Ok(file.to_string())
}
