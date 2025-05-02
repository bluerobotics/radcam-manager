use serde::{Deserialize, Serialize};
use serde_repr::*;
use serde_with::skip_serializing_none;
use ts_rs::TS;
use tsync::tsync;

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayMirrorValue {
    #[default]
    Open = 0,
    Close = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayFlipValue {
    #[default]
    Open = 0,
    Close = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayPowerFreqValue {
    #[default]
    NTSC = 0,
    PAL = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayColorBlackValue {
    #[default]
    Color = 0,
    Auto = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayInfrDetectModeValue {
    #[default]
    VideoDetection = 0,
    TimeControl = 1,
    PhotosensitiveDetection = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayLensCorrectionValue {
    #[default]
    Open = 0,
    Close = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayIRCUTLevelValue {
    #[default]
    LowLevel = 0,
    HighLevel = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayLDRLevelValue {
    #[default]
    LowLevel = 0,
    HighLevel = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayAntiflickerValue {
    #[default]
    Close = 0,
    Auto = 1,
    _50HZ = 2,
    _60HZ = 3,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplaySceneModeValue {
    #[default]
    IPC = 0,
    FaceCapture = 1,
    LicensePlateCapture = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayHlcEnableValue {
    #[default]
    Close = 0,
    Open = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayLowFramerateValue {
    #[default]
    Close = 0,
    Open = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplay2dNrLevelValue {
    #[default]
    Low = 0,
    Middle = 1,
    High = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayWDRSensorValue {
    #[default]
    Close = 0,
    Open = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayNoiseReductionValue {
    #[default]
    Close = 0,
    Low = 1,
    Middle = 2,
    High = 3,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayAutoIrisValue {
    #[default]
    Open = 0,
    Close = 1,
    Manual = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayLedControlValue {
    #[default]
    Auto = 0,
    Open = 1,
    Close = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayLedControlAvailValue {
    #[default]
    LowLevel = 0,
    HighLevel = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayLightControlModeValue {
    #[default]
    ElectricalLevel = 0,
    PWM = 1,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[tsync]
#[repr(u8)]
pub enum AdvancedDisplayLampTypeValue {
    #[default]
    InfraredLamp = 0,
    WhiteLight = 1,
    Auto = 2,
}

#[skip_serializing_none]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct AdvancedParameterSetting {
    /// Mirror.
    pub mirror: Option<AdvancedDisplayMirrorValue>,
    /// Flip.
    pub flip: Option<AdvancedDisplayFlipValue>,
    /// Video Format.
    pub power_freq: Option<AdvancedDisplayPowerFreqValue>,
    /// Color Turned Black.
    pub color_black: Option<AdvancedDisplayColorBlackValue>,
    /// Video Detection Mode. Only support when `color_black` is `1`.
    pub infr_detect_mode: Option<AdvancedDisplayInfrDetectModeValue>,
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
    pub lens_correction: Option<AdvancedDisplayLensCorrectionValue>,
    /// Wide Dynamic Strength. Range: [0..=255].
    pub wdr_level: Option<u8>,
    /// IRCUT Level.
    pub ircut_level: Option<AdvancedDisplayIRCUTLevelValue>,
    /// Photosensitive Level.
    pub ldr_level: Option<AdvancedDisplayLDRLevelValue>,
    /// Light Pattern.
    pub led_control_mode: Option<AdvancedDisplayLightControlModeValue>,
    // Light Type.
    pub lamp_type: Option<AdvancedDisplayLampTypeValue>,
    /// Light Enable Level. Only support when `led_control_mode` is `0`.
    pub led_control_avail: Option<AdvancedDisplayLedControlAvailValue>,
    /// Infrared Lamp Brightness. Only support when `led_control_mode` is `1`. Range: [0..=255].
    pub ir_level: Option<u8>,
    /// White Light Brightness. Only support when `lamp_type` is `1` and `led_control_mode` is `1`. Range: [0..=255].
    pub led_level: Option<u8>,
    /// IR Control.
    pub led_control: Option<AdvancedDisplayLedControlValue>,
    /// Aperture mode.
    pub auto_iris: Option<AdvancedDisplayAutoIrisValue>,
    /// Control the duty cycle of aperture PWM. Only support when `auto_iris` is `2`. Range: [0..=255]. Not Settable.
    #[serde(rename = "irisLevel")]
    #[serde(alias = "iris_level")]
    pub iris_level: Option<u8>,
    /// 3D Noise Reduction.
    #[serde(rename = "noiseReduction")]
    #[serde(alias = "noise_reduction")]
    pub noise_reduction: Option<AdvancedDisplayNoiseReductionValue>,
    /// WDR Enable.
    pub wdr_sensor: Option<AdvancedDisplayWDRSensorValue>,
    /// WDR Strength. Range: [0..=255].
    pub wdr_level_sensor: Option<u8>,
    /// HLC.
    pub hlc_enable: Option<AdvancedDisplayHlcEnableValue>,
    /// Slow Shutter.
    pub low_farme_rate: Option<AdvancedDisplayLowFramerateValue>,
    /// 2D NR.
    #[serde(rename = "_2DNR_level")]
    #[serde(alias = "_2d_nr_level")]
    pub _2d_nr_level: Option<AdvancedDisplay2dNrLevelValue>,
    /// Anti Flicker
    pub anti_flicker: Option<AdvancedDisplayAntiflickerValue>,
    /// Scene Mode
    pub scene_mode: Option<AdvancedDisplaySceneModeValue>,
    /// [Custom] Automatic White Balance trigger. Range: [0..=1].
    #[serde(rename = "onceAWB")]
    #[serde(alias = "once_awb")]
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
            mirror: Some(AdvancedDisplayMirrorValue::Open),
            flip: Some(AdvancedDisplayFlipValue::Open),
            power_freq: Some(AdvancedDisplayPowerFreqValue::PAL),
            color_black: Some(AdvancedDisplayColorBlackValue::Auto),
            lens_correction: Some(AdvancedDisplayLensCorrectionValue::Open),
            wdr_level: Some(128),
            ircut_level: Some(AdvancedDisplayIRCUTLevelValue::LowLevel),
            ldr_level: Some(AdvancedDisplayLDRLevelValue::HighLevel),
            ..Default::default()
        };

        assert_eq!(expected_params, params);
    }
}
