use anyhow::Result;
use mlua::Lua;
use tera::{Context, Tera};

pub fn generate_lua_script(config: &crate::ZoomAndFocusConfig) -> Result<String> {
    let mut context = Context::new();

    context.insert("k_focus", &config.k_focus);
    context.insert("k_zoom", &config.k_zoom);
    context.insert("k_scripting1", &config.k_scripting1);
    context.insert("margin_gain", &config.margin_gain);
    context.insert("closest_points", &config.closest_points.to_lua());
    context.insert("furthest_points", &config.furthest_points.to_lua());
    context.insert("focus_channel", &config.focus_channel);
    context.insert("zoom_channel", &config.zoom_channel);
    context.insert("custom1_channel", &config.custom1_channel);
    context.insert("zoom_output_pwm", &config.zoom_output_pwm);
    context.insert("zoom_range", &config.zoom_range);
    context.insert("zoom_scaled", &config.zoom_scaled);

    let template = r#"
--- Focus correction script.

-- Usage: change the output controlling focus from CameraFocus to "Script1"
--        but assign CameraFocus to any other "Disabled" channel as we need to read it, as it allows fine-tuning

K_FOCUS = {{ k_focus }}
K_ZOOM = {{ k_zoom }}
K_SCRIPTING1 = {{ k_scripting1 }}

local MARGIN_GAIN = {{ margin_gain }}   -- this will allow us to move 5% beyeond closest/furthest points

-- Lookup tables for closest and furthest focus points
local closest_points = {{ closest_points }}

local furthest_points = {{ furthest_points }}

local focus_channel = SRV_Channels:find_channel({{ focus_channel }})
local zoom_channel = SRV_Channels:find_channel({{ zoom_channel }})
local custom1_channel = SRV_Channels:find_channel({{ custom1_channel }})

-- set zoom to trim levels
SRV_Channels:set_output_pwm(zoom_channel, {{ zoom_output_pwm }})
SRV_Channels:set_range(zoom_channel, {{ zoom_range }})
SRV_Channels:set_output_scaled(zoom_channel, {{ zoom_scaled }})

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

    let res = Tera::one_off(template, &context, false)?;

    Ok(res)
}

pub fn validate_lua(script: &str) -> Result<()> {
    let lua = Lua::new();
    lua.load(script).into_function()?;

    Ok(())
}
