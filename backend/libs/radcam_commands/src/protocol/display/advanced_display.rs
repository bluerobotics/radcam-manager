use serde::{Deserialize, Serialize};
use serde_repr::*;
use serde_with::skip_serializing_none;
use ts_rs::TS;

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum MirrorValue {
    #[default]
    Open = 0,
    Close = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum FlipValue {
    #[default]
    Open = 0,
    Close = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum PowerFreqValue {
    #[default]
    NTSC = 0,
    PAL = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum ColorBlackValue {
    #[default]
    Color = 0,
    Auto = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum InfrDetectModeValue {
    #[default]
    VideoDetection = 0,
    TimeControl = 1,
    PhotosensitiveDetection = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum LensCorrectionValue {
    #[default]
    Open = 0,
    Close = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum IRCUTLevel {
    #[default]
    LowLevel = 0,
    HighLevel = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum LDRLevel {
    #[default]
    LowLevel = 0,
    HighLevel = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum Antiflicker {
    #[default]
    Close = 0,
    Auto = 1,
    _50HZ = 2,
    _60HZ = 3,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum Scenemode {
    #[default]
    IPC = 0,
    FaceCapture = 1,
    LicensePlateCapture = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum Hlcenable {
    #[default]
    Close = 0,
    Open = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum Lowfarmerate {
    #[default]
    Close = 0,
    Open = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum _2dNrLevel {
    #[default]
    Low = 0,
    Middle = 1,
    High = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum WDRSensor {
    #[default]
    Close = 0,
    Open = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum NoiseReduction {
    #[default]
    Close = 0,
    Low = 1,
    Middle = 2,
    High = 3,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum AutoIris {
    #[default]
    Open = 0,
    Close = 1,
    Manual = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum LedControl {
    #[default]
    Auto = 0,
    Open = 1,
    Close = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum LedControlAvail {
    #[default]
    LowLevel = 0,
    HighLevel = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum LightControlMode {
    #[default]
    ElectricalLevel = 0,
    PWM = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum LampType {
    #[default]
    InfraredLamp = 0,
    WhiteLight = 1,
    Auto = 2,
}

#[skip_serializing_none]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct AdvancedParameterSetting {
    /// Mirror.
    pub mirror: Option<MirrorValue>,
    /// Flip.
    pub flip: Option<FlipValue>,
    /// Video Format.
    pub power_freq: Option<PowerFreqValue>,
    /// Color Turned Black.
    pub color_black: Option<ColorBlackValue>,
    /// Video Detection Mode. Only support when `color_black` is `1`.
    pub infr_detect_mode: Option<InfrDetectModeValue>,
    /// Color To Black Sensitivity. Only support when `infr_detect_mode` is `0`. Range: [0..=255].
    pub sens_day_to_night: Option<u8>,
    /// Black To Color Sensitivity. Only support when `infr_detect_mode` is `0`. Range: [0..=255].
    pub sens_night_to_day: Option<u8>,
    /// Color Turned (Time Control) Start Time Hour. Only support when `infr_detect_mode` is `1`. Range: [0..=23].
    pub infr_day_h: Option<u8>,
    /// Color Turned (Time Control) Start Time Min. Only support when `infr_detect_mode` is `1`. Range: [0..=59].
    pub infr_day_m: Option<u8>,
    /// Color Turned (Time Control) End Time Hour. Only support when `infr_detect_mode` is `1`. Range: [0..=23].
    pub infr_night_h: Option<u8>,
    /// Color Turned (Time Control) End Time Min. Only support when `infr_detect_mode` is `1`. Range: [0..=59].
    pub infr_night_m: Option<u8>,
    /// Lens Correction. Range: [0..=255].
    pub lens_correction: Option<LensCorrectionValue>,
    /// Wide Dynamic Strength. Range: [0..=255].
    pub wdr_level: Option<u8>,
    /// IRCUT Level.
    pub ircut_level: Option<IRCUTLevel>,
    /// Photosensitive Level.
    pub ldr_level: Option<LDRLevel>,
    /// Light Pattern.
    pub led_control_mode: Option<LightControlMode>,
    // Light Type.
    pub lamp_type: Option<LampType>,
    /// Light Enable Level. Only support when `led_control_mode` is `0`.
    pub led_control_avail: Option<LedControlAvail>,
    /// Infrared Lamp Brightness. Only support when `led_control_mode` is `1`. Range: [0..=255].
    pub ir_level: Option<u8>,
    /// White Light Brightness. Only support when `lamp_type` is `1` and `led_control_mode` is `1`. Range: [0..=255].
    pub led_level: Option<u8>,
    /// IR Control.
    pub led_control: Option<LedControl>,
    /// Aperture mode.
    pub auto_iris: Option<AutoIris>,
    /// Control the duty cycle of aperture PWM. Only support when `auto_iris` is `2`. Range: [0..=255]. Not Settable.
    #[serde(rename = "irisLevel")]
    pub iris_level: Option<u8>,
    /// 3D Noise Reduction.
    #[serde(rename = "noiseReduction")]
    pub noise_reduction: Option<NoiseReduction>,
    /// WDR Enable.
    pub wdr_sensor: Option<WDRSensor>,
    /// WDR Strength. Range: [0..=255].
    pub wdr_level_sensor: Option<u8>,
    /// HLC.
    pub hlc_enable: Option<Hlcenable>,
    /// Slow Shutter.
    pub low_farme_rate: Option<Lowfarmerate>,
    /// 2D NR.
    #[serde(rename = "_2DNR_level")]
    pub _2d_nr_level: Option<_2dNrLevel>,
    /// Anti Flicker
    pub anti_flicker: Option<Antiflicker>,
    /// Scene Mode
    pub scene_mode: Option<Scenemode>,
    /// [Custom] Automatic White Balance trigger. Range: [0..=1].
    #[serde(rename = "onceAWB")]
    pub once_awb: Option<u8>,
    /// Restores all AdvancedParameterSetting parameters when `1`
    pub set_default: Option<u8>,
}

#[cfg(test)]
mod tests {

    use serde_json::json;

    use utils::deserialize;

    use super::*;

    #[test]
    fn deserialize_test() {
        let json = r##"{
              "flip": 0,
              "mirror": 0,
              "color_black": 1,
              "noiseReduction": 1,
              "lens_correction": 0,
              "byLDC_XOffset": 0,
              "byLDC_YOffset": 50,
              "byLDC_Ratio": 200,
              "auto_iris": 1,
              "wdr_level": 128,
              "power_freq": 0,
              "irisLevel": 2,
              "ircut_level": 0,
              "ldr_level": 1,
              "led_control": 0,
              "led_control_avail": 1,
              "led_control_avail": 1,
              "led_level": 48,
              "white_control": 0,
              "ir_level": 48,
              "night2day_level": 0,
              "day2night_level": 0,
              "lamp_type": 0,
              "led_control_mode": 0,
              "infr_detect_mode": 2,
              "infr_day_h": 7,
              "infr_day_m": 0,
              "infr_night_h": 18,
              "infr_night_m": 0,
              "sens_day_to_night": 255,
              "sens_night_to_day": 160,
              "led_open_level": 0,
              "led_close_level": 0,
              "hlc_enable": 0,
              "low_farme_rate": 1,
              "_2DNR_level": 0,
              "anti_flicker": 0,
              "onceAWB": 0,
              "scene_mode": 0,
              "code": 0,
              "device_mac": "bc-07-18-01-c5-0f",
              "deviceID": "H01000118160100011616",
              "device_id": "H01000118160100011616",
              "log": "",
              "device_ip": "192.168.0.106",
              "sign_tby": "b14701e44da7d83b064a974cf61a4a6c"
            }"##
        .to_string();

        deserialize::<AdvancedParameterSetting>(&json).expect("Failed deserializing");
    }

    #[test]
    fn setting_some_parameters() {
        let json = json!({
            "flip": 0,
            "mirror": 0,
            "color_black": 1,
            "lens_correction": 0,
            "wdr_level": 128,
            "power_freq": 1,
            "ircut_level": 0,
            "ldr_level": 1
        })
        .to_string();

        let params = deserialize::<AdvancedParameterSetting>(&json).expect("Failed deserializing");

        let expected_params = AdvancedParameterSetting {
            mirror: Some(MirrorValue::Open),
            flip: Some(FlipValue::Open),
            power_freq: Some(PowerFreqValue::PAL),
            color_black: Some(ColorBlackValue::Auto),
            lens_correction: Some(LensCorrectionValue::Open),
            wdr_level: Some(128),
            ircut_level: Some(IRCUTLevel::LowLevel),
            ldr_level: Some(LDRLevel::HighLevel),
            ..Default::default()
        };

        assert_eq!(expected_params, params);
    }
}
