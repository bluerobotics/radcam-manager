use crate::{FocusZoomPoint, FocusZoomPoints, ZoomAndFocusConfig};

impl Default for ZoomAndFocusConfig {
    fn default() -> Self {
        Self {
            k_focus: K_FOCUS,
            k_zoom: K_ZOOM,
            k_scripting1: K_SCRIPTING1,
            margin_gain: MARGIN_GAIN,
            closest_points: FocusZoomPoints(CLOSEST_POINTS.to_vec()),
            furthest_points: FocusZoomPoints(FURTHEST_POINTS.to_vec()),
            focus_channel: FOCUS_CHANNEL,
            zoom_channel: ZOOM_CHANNEL,
            custom1_channel: CUSTOM1_CHANNEL,
            zoom_output_pwm: ZOOM_OUTPUT_PWM,
            zoom_range: ZOOM_RANGE,
            zoom_scaled: ZOOM_SCALED,
        }
    }
}

pub const K_FOCUS: u32 = 92;
pub const K_ZOOM: u32 = 180;
pub const K_SCRIPTING1: u32 = 94;
pub const MARGIN_GAIN: f32 = 1.05;
pub const FOCUS_CHANNEL: u32 = 92;
pub const ZOOM_CHANNEL: u32 = 180;
pub const CUSTOM1_CHANNEL: u32 = 10;
pub const ZOOM_OUTPUT_PWM: u32 = 1000;
pub const ZOOM_RANGE: u32 = 1000;
pub const ZOOM_SCALED: u32 = 0;

pub const CLOSEST_POINTS: &'static [FocusZoomPoint] = &[
    FocusZoomPoint {
        zoom: 900,
        focus: 882,
    },
    FocusZoomPoint {
        zoom: 1100,
        focus: 1253,
    },
    FocusZoomPoint {
        zoom: 1300,
        focus: 1498,
    },
    FocusZoomPoint {
        zoom: 1500,
        focus: 1669,
    },
    FocusZoomPoint {
        zoom: 1700,
        focus: 1759,
    },
    FocusZoomPoint {
        zoom: 1900,
        focus: 1862,
    },
    FocusZoomPoint {
        zoom: 2100,
        focus: 1883,
    },
];
pub const FURTHEST_POINTS: &'static [FocusZoomPoint] = &[
    FocusZoomPoint {
        zoom: 900,
        focus: 935,
    },
    FocusZoomPoint {
        zoom: 1100,
        focus: 1305,
    },
    FocusZoomPoint {
        zoom: 1300,
        focus: 1520,
    },
    FocusZoomPoint {
        zoom: 1500,
        focus: 1696,
    },
    FocusZoomPoint {
        zoom: 1700,
        focus: 1811,
    },
    FocusZoomPoint {
        zoom: 1900,
        focus: 1911,
    },
    FocusZoomPoint {
        zoom: 2100,
        focus: 1930,
    },
];
