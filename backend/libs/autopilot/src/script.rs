use anyhow::Result;
use mlua::Lua;
use tera::{Context, Tera};

use crate::parameters::{FOCUS_CHANNEL_FUNCTION, ZOOM_CHANNEL_FUNCTION};

pub const PARAM_TABLE_KEY: u8 = 72;
pub const PARAM_PREFIX: &'static str = "\"FOCUS_\"";

pub fn generate_lua_script(config: &crate::ZoomAndFocusConfig) -> Result<String> {
    let mut context = Context::new();

    let focus_channel_function = FOCUS_CHANNEL_FUNCTION;
    let zoom_channel_function = ZOOM_CHANNEL_FUNCTION;
    let param_table_key = PARAM_TABLE_KEY;
    let param_prefix = PARAM_PREFIX;

    context.insert("param_table_key", &param_table_key);
    context.insert("param_prefix", param_prefix);
    context.insert("margin_gain", &(config.parameters.focus_margin_gain as f32));
    context.insert("k_focus", &focus_channel_function);
    context.insert("k_zoom", &zoom_channel_function);
    context.insert("k_script", &(config.parameters.focus_script_function as u8));
    context.insert(
        "enable_focus_and_zoom_correlation",
        &(config.parameters.enable_focus_and_zoom_correlation as u8),
    );
    context.insert("closest_points", &config.closest_points.to_lua());
    context.insert("furthest_points", &config.furthest_points.to_lua());

    let template = r#"
--- Focus correction script.


-- Usage: change the output controlling focus from CameraFocus to "Script1"
--        but assign CameraFocus to any other "Disabled" channel as we need to read it, as it allows fine-tuning

-- Unique key (0-200). Must not conflict with other scripts
local PARAM_TABLE_KEY = {{ param_table_key }}

-- Prefix for all parameter names in this table
local PARAM_PREFIX = {{ param_prefix }}

-- Add parameter table with N parameters
assert(param:add_table(PARAM_TABLE_KEY, PARAM_PREFIX, 5), 'could not add param table')

-- Define parameters with index (1-63), name, and default value
assert(param:add_param(PARAM_TABLE_KEY, 1, "MARGIN_GAIN", {{ margin_gain }}), 'could not add MARGIN_GAIN')
assert(param:add_param(PARAM_TABLE_KEY, 2, "K_FOCUS", {{ k_focus }}), 'could not add K_FOCUS')
assert(param:add_param(PARAM_TABLE_KEY, 3, "K_ZOOM", {{ k_zoom }}), 'could not add K_ZOOM')
assert(param:add_param(PARAM_TABLE_KEY, 4, "K_SCRIPT", {{ k_script }}), 'could not add K_SCRIPT')
assert(param:add_param(PARAM_TABLE_KEY, 5, "ENABLE", {{ enable_focus_and_zoom_correlation }}), 'could not add K_SCRIPT')

-- Initialize parameter objects
local MARGIN_GAIN = Parameter()  -- this will allow us to move 5% beyeond closest/furthest focus lines, useful for making a new curve
local K_FOCUS = Parameter()
local K_ZOOM = Parameter()
local K_SCRIPT = Parameter()
local ENABLE = Parameter()

-- Bind them to physical parameter locations
assert(MARGIN_GAIN:init(PARAM_PREFIX .. "MARGIN_GAIN"),   "Failed to init MARGIN_GAIN")
assert(K_FOCUS:init(PARAM_PREFIX .. "K_FOCUS"),           "Failed to init K_FOCUS")
assert(K_ZOOM:init(PARAM_PREFIX .. "K_ZOOM"),             "Failed to init K_ZOOM")
assert(K_SCRIPT:init(PARAM_PREFIX .. "K_SCRIPT"), "Failed to init K_SCRIPT")
assert(ENABLE:init(PARAM_PREFIX .. "ENABLE"), "Failed to init ENABLE")

-- Lookup tables for closest and furthest focus points
local closest_points = {{ closest_points }}

local furthest_points = {{ furthest_points }}


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
local function calculate_focus(focus, zoom, margin_gain)
    -- Interpolate both closest and furthest focus values
    local closest_focus = interpolate_focus(zoom, closest_points)
    local furthest_focus = interpolate_focus(zoom, furthest_points)

    -- Linear interpolation between closest and furthest based on focus_delta
    local focus_delta = 0.5 + (focus - 1500) / 400
    local range = (furthest_focus - closest_focus) * margin_gain
    local mid_point = closest_focus + range / 2
    return math.floor(mid_point + (focus_delta - 0.5) * range)
end

local tick = 0
function update()
    -- Retrieve values from parameters
    local margin_gain = MARGIN_GAIN:get()
    local k_focus = K_FOCUS:get()
    local k_zoom = K_ZOOM:get()
    local k_script = K_SCRIPT:get()
    local enable_focus_and_zoom_correlation = ENABLE:get()

    local focus_input = SRV_Channels:get_output_pwm(k_focus)
    local zoom_input = SRV_Channels:get_output_pwm(k_zoom)

    local focus_output
    if enable_focus_and_zoom_correlation then
        focus_output = calculate_focus(focus_input, zoom_input, margin_gain)
    else
        focus_output = focus_input
    end

    tick = tick + 1
    if tick % 10 == 0 then  -- every ~1s
        gcs:send_text(0, string.format(
            "Params: margin_gain=%.2f k_focus=%d k_zoom=%d k_script=%d focus_input=%d zoom_input=%d focus_output=%d",
            margin_gain, k_focus, k_zoom, k_script, focus_input, zoom_input, focus_output
        ))
    end

    SRV_Channels:set_output_pwm(k_script, focus_output)
   
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
