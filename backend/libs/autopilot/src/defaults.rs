use crate::{
    FocusZoomPoint, FocusZoomPoints, ZoomAndFocusConfig,
    parameters::{
        FocusAndZoomParameters, MountType, ScriptFunction, ServoChannel, TiltChannelFunction,
    },
};

impl Default for ZoomAndFocusConfig {
    fn default() -> Self {
        Self {
            parameters: FocusAndZoomParameters::default(),
            closest_points: FocusZoomPoints(CLOSEST_POINTS.to_vec()),
            furthest_points: FocusZoomPoints(FURTHEST_POINTS.to_vec()),
        }
    }
}

impl Default for FocusAndZoomParameters {
    fn default() -> Self {
        Self {
            // Focus, controlled by the user
            focus_channel: ServoChannel::SERVO10,
            focus_channel_min: 870,
            focus_channel_trim: 1500,
            focus_channel_max: 2130,
            focus_margin_gain: 1.0,

            // TODO: figure out how to configure this correclty
            // script_channel: ServoChannel::SERVO10,
            focus_script_function: ScriptFunction::SCRIPT1,
            enable_focus_and_zoom_correlation: true,

            // Zoom, controlled by the user
            zoom_channel: ServoChannel::SERVO11,
            zoom_channel_min: 935,
            zoom_channel_trim: 1500,
            zoom_channel_max: 1850,

            // Tilt
            tilt_channel: ServoChannel::SERVO16,
            tilt_channel_min: 2250,
            tilt_channel_trim: 1500,
            tilt_channel_max: 750,
            tilt_channel_reversed: false,

            // Mount
            tilt_mnt_type: MountType::BrushlessPWM,
            tilt_mnt_pitch_min: -90,
            tilt_mnt_pitch_max: 90,
        }
    }
}

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
