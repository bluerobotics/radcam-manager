use anyhow::{Result, anyhow};
use mavlink::ardupilotmega::{MavParamType, PARAM_VALUE_DATA};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use ts_rs::TS;

use crate::mavlink::ParamEncodingType;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Parameter {
    pub name: String,
    pub value: ParamType,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ParamType {
    UINT8(u8),
    INT8(i8),
    UINT16(u16),
    INT16(i16),
    UINT32(u32),
    INT32(i32),
    UINT64(u64),
    INT64(i64),
    REAL32(f32),
    REAL64(f64),
}

impl ParamType {
    pub fn encode(&self, encoding: ParamEncodingType) -> Result<f32> {
        use ParamEncodingType::*;

        let value = match (self, encoding) {
            // C_CAST
            (ParamType::UINT8(v), CCast) => *v as f32,
            (ParamType::INT8(v), CCast) => *v as f32,
            (ParamType::UINT16(v), CCast) => *v as f32,
            (ParamType::INT16(v), CCast) => *v as f32,
            (ParamType::UINT32(v), CCast) => *v as f32,
            (ParamType::INT32(v), CCast) => *v as f32,
            (ParamType::REAL32(v), CCast) => *v,
            (ParamType::REAL64(v), CCast) => *v as f32,
            (ParamType::UINT64(_) | ParamType::INT64(_), CCast) => {
                panic!("Use PARAM_EXT_* for 64-bit values")
            }

            // ByteWise
            (ParamType::UINT8(v), ByteWise) => f32::from_bits(*v as u32),
            (ParamType::INT8(v), ByteWise) => f32::from_bits(*v as u8 as u32),
            (ParamType::UINT16(v), ByteWise) => f32::from_bits(*v as u32),
            (ParamType::INT16(v), ByteWise) => f32::from_bits(*v as u16 as u32),
            (ParamType::UINT32(v), ByteWise) => f32::from_bits(*v),
            (ParamType::INT32(v), ByteWise) => f32::from_bits(*v as u32),
            (ParamType::REAL32(v), ByteWise) => *v,
            (ParamType::UINT64(_) | ParamType::INT64(_) | ParamType::REAL64(_), ByteWise) => {
                panic!("Use PARAM_EXT_* for 64-bit values")
            }

            // Unsupported
            (_, Unsupported) => return Err(anyhow!("Unsupported encoding")),
        };

        Ok(value)
    }

    fn decode(data: &PARAM_VALUE_DATA, encoding: ParamEncodingType) -> Result<Self> {
        use MavParamType::*;
        use ParamEncodingType::*;

        let param = match (data.param_type, encoding) {
            // C_CAST
            (MAV_PARAM_TYPE_UINT8, CCast) => ParamType::UINT8(data.param_value as u8),
            (MAV_PARAM_TYPE_INT8, CCast) => ParamType::INT8(data.param_value as i8),
            (MAV_PARAM_TYPE_UINT16, CCast) => ParamType::UINT16(data.param_value as u16),
            (MAV_PARAM_TYPE_INT16, CCast) => ParamType::INT16(data.param_value as i16),
            (MAV_PARAM_TYPE_UINT32, CCast) => ParamType::UINT32(data.param_value as u32),
            (MAV_PARAM_TYPE_INT32, CCast) => ParamType::INT32(data.param_value as i32),
            (MAV_PARAM_TYPE_REAL32, CCast) => ParamType::REAL32(data.param_value),
            (MAV_PARAM_TYPE_REAL64, CCast) => ParamType::REAL64(data.param_value as f64),
            (MAV_PARAM_TYPE_UINT64 | MAV_PARAM_TYPE_INT64, CCast) => {
                panic!("Use PARAM_EXT_* for 64-bit values")
            }

            // ByteWise
            (MAV_PARAM_TYPE_UINT8, ByteWise) => ParamType::UINT8(data.param_value.to_bits() as u8),
            (MAV_PARAM_TYPE_INT8, ByteWise) => ParamType::INT8(data.param_value.to_bits() as i8),
            (MAV_PARAM_TYPE_UINT16, ByteWise) => {
                ParamType::UINT16(data.param_value.to_bits() as u16)
            }
            (MAV_PARAM_TYPE_INT16, ByteWise) => ParamType::INT16(data.param_value.to_bits() as i16),
            (MAV_PARAM_TYPE_UINT32, ByteWise) => ParamType::UINT32(data.param_value.to_bits()),
            (MAV_PARAM_TYPE_INT32, ByteWise) => ParamType::INT32(data.param_value.to_bits() as i32),
            (MAV_PARAM_TYPE_REAL32, ByteWise) => {
                ParamType::REAL32(f32::from_bits(data.param_value.to_bits()))
            }

            // 64-bit and REAL64 require the *extended* protocol
            (MAV_PARAM_TYPE_UINT64 | MAV_PARAM_TYPE_INT64 | MAV_PARAM_TYPE_REAL64, ByteWise) => {
                panic!("Use PARAM_EXT_* for 64-bit values")
            }

            (_, Unsupported) => return Err(anyhow!("Unsupported encoding")),
        };

        Ok(param)
    }

    pub fn set_value(&mut self, new_value: ParamType, encoding: ParamEncodingType) -> Result<()> {
        let encoded_value = new_value.encode(encoding)?;

        use ParamEncodingType::*;
        use ParamType::*;

        match (self, encoding) {
            // C_CAST
            (UINT8(value), CCast) => *value = encoded_value as u8,
            (INT8(value), CCast) => *value = encoded_value as i8,
            (UINT16(value), CCast) => *value = encoded_value as u16,
            (INT16(value), CCast) => *value = encoded_value as i16,
            (UINT32(value), CCast) => *value = encoded_value as u32,
            (INT32(value), CCast) => *value = encoded_value as i32,
            (UINT64(value), CCast) => *value = encoded_value as u64,
            (INT64(value), CCast) => *value = encoded_value as i64,
            (REAL32(value), CCast) => *value = encoded_value,
            (REAL64(value), CCast) => *value = encoded_value as f64,

            // ByteWise
            (UINT8(value), ByteWise) => *value = encoded_value.to_bits() as u8,
            (INT8(value), ByteWise) => *value = encoded_value.to_bits() as i8,
            (UINT16(value), ByteWise) => *value = encoded_value.to_bits() as u16,
            (INT16(value), ByteWise) => *value = encoded_value.to_bits() as i16,
            (UINT32(value), ByteWise) => *value = encoded_value.to_bits() as u32,
            (INT32(value), ByteWise) => *value = encoded_value.to_bits() as i32,
            (UINT64(value), ByteWise) => *value = encoded_value.to_bits() as u64,
            (INT64(value), ByteWise) => *value = encoded_value.to_bits() as i64,
            (REAL32(value), ByteWise) => *value = encoded_value,
            (REAL64(value), ByteWise) => *value = encoded_value.to_bits() as f64,

            //
            (_, Unsupported) => return Err(anyhow!("Unsupported encoding")),
        }

        Ok(())
    }
}

impl Parameter {
    pub fn try_new(data: &PARAM_VALUE_DATA, encoding: ParamEncodingType) -> Result<Self> {
        Ok(Self {
            name: Self::param_id_to_name(data.param_id),
            value: ParamType::decode(data, encoding)?,
        })
    }

    pub fn param_value(&self, encoding: ParamEncodingType) -> Result<f32> {
        self.value.encode(encoding)
    }

    pub fn param_type(&self) -> MavParamType {
        match &self.value {
            ParamType::UINT8(_) => MavParamType::MAV_PARAM_TYPE_UINT8,
            ParamType::INT8(_) => MavParamType::MAV_PARAM_TYPE_INT8,
            ParamType::UINT16(_) => MavParamType::MAV_PARAM_TYPE_UINT16,
            ParamType::INT16(_) => MavParamType::MAV_PARAM_TYPE_INT16,
            ParamType::UINT32(_) => MavParamType::MAV_PARAM_TYPE_UINT32,
            ParamType::INT32(_) => MavParamType::MAV_PARAM_TYPE_INT32,
            ParamType::UINT64(_) => MavParamType::MAV_PARAM_TYPE_UINT64,
            ParamType::INT64(_) => MavParamType::MAV_PARAM_TYPE_INT64,
            ParamType::REAL32(_) => MavParamType::MAV_PARAM_TYPE_REAL32,
            ParamType::REAL64(_) => MavParamType::MAV_PARAM_TYPE_REAL64,
        }
    }

    pub fn param_id(&self) -> [u8; 16] {
        Self::param_name_to_id(&self.name)
    }

    pub fn param_id_to_name(id: [u8; 16]) -> String {
        let len = id.iter().position(|&b| b == 0).unwrap_or(16);
        String::from_utf8_lossy(&id[..len]).to_string()
    }

    pub fn param_name_to_id(name: &str) -> [u8; 16] {
        let mut buffer = [0u8; 16];
        let bytes = name.as_bytes();
        let len = bytes.len().min(16);
        buffer[..len].copy_from_slice(&bytes[..len]);
        buffer
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FocusAndZoomParameters {
    // Focus channel parameters
    pub focus_channel: ServoChannel,
    pub focus_channel_min: u16,
    pub focus_channel_trim: u16,
    pub focus_channel_max: u16,
    pub focus_margin_gain: f32,
    pub focus_script_function: ScriptFunction,
    pub enable_focus_and_zoom_correlation: bool,

    // Zoom channel parameters
    pub zoom_channel: ServoChannel,
    pub zoom_channel_min: u16,
    pub zoom_channel_trim: u16,
    pub zoom_channel_max: u16,

    // Tilt channel parameters
    pub tilt_channel: ServoChannel,
    pub tilt_channel_min: u16,
    pub tilt_channel_trim: u16,
    pub tilt_channel_max: u16,
    pub tilt_channel_reversed: bool,

    // Mount (MNTx) parameters
    pub tilt_mnt_type: MountType,
    pub tilt_mnt_pitch_min: i32,
    pub tilt_mnt_pitch_max: i32,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, TS)]
pub struct FocusAndZoomParametersQuery {
    // Focus channel parameters
    pub focus_channel: Option<ServoChannel>,
    pub focus_channel_min: Option<u16>,
    pub focus_channel_trim: Option<u16>,
    pub focus_channel_max: Option<u16>,
    pub focus_margin_gain: Option<f32>,
    pub script_function: Option<ScriptFunction>,
    pub enable_focus_and_zoom_correlation: Option<bool>,

    // Zoom channel parameters
    pub zoom_channel: Option<ServoChannel>,
    pub zoom_channel_min: Option<u16>,
    pub zoom_channel_trim: Option<u16>,
    pub zoom_channel_max: Option<u16>,

    // Tilt channel parameters
    pub tilt_channel: Option<ServoChannel>,
    pub tilt_channel_min: Option<u16>,
    pub tilt_channel_trim: Option<u16>,
    pub tilt_channel_max: Option<u16>,
    pub tilt_channel_reversed: Option<bool>,

    // Mount (MNTx) parameters
    pub tilt_mnt_type: Option<MountType>,
    pub tilt_mnt_pitch_min: Option<i32>,
    pub tilt_mnt_pitch_max: Option<i32>,
}

impl From<FocusAndZoomParameters> for FocusAndZoomParametersQuery {
    fn from(value: FocusAndZoomParameters) -> Self {
        Self {
            focus_channel: Some(value.focus_channel),
            focus_channel_min: Some(value.focus_channel_min),
            focus_channel_trim: Some(value.focus_channel_trim),
            focus_channel_max: Some(value.focus_channel_max),
            focus_margin_gain: Some(value.focus_margin_gain),
            script_function: Some(value.focus_script_function),
            enable_focus_and_zoom_correlation: Some(value.enable_focus_and_zoom_correlation),
            zoom_channel: Some(value.zoom_channel),
            zoom_channel_min: Some(value.zoom_channel_min),
            zoom_channel_trim: Some(value.zoom_channel_trim),
            zoom_channel_max: Some(value.zoom_channel_max),
            tilt_channel: Some(value.tilt_channel),
            tilt_channel_min: Some(value.tilt_channel_min),
            tilt_channel_trim: Some(value.tilt_channel_trim),
            tilt_channel_max: Some(value.tilt_channel_max),
            tilt_channel_reversed: Some(value.tilt_channel_reversed),
            tilt_mnt_type: Some(value.tilt_mnt_type),
            tilt_mnt_pitch_min: Some(value.tilt_mnt_pitch_min),
            tilt_mnt_pitch_max: Some(value.tilt_mnt_pitch_max),
        }
    }
}

impl From<FocusAndZoomParametersQuery> for FocusAndZoomParameters {
    fn from(value: FocusAndZoomParametersQuery) -> Self {
        let default = Self::default();
        Self {
            focus_channel: value.focus_channel.unwrap_or(default.focus_channel),
            focus_channel_min: value.focus_channel_min.unwrap_or(default.focus_channel_min),
            focus_channel_trim: value
                .focus_channel_trim
                .unwrap_or(default.focus_channel_trim),
            focus_channel_max: value.focus_channel_max.unwrap_or(default.focus_channel_max),
            focus_margin_gain: value.focus_margin_gain.unwrap_or(default.focus_margin_gain),
            focus_script_function: value
                .script_function
                .unwrap_or(default.focus_script_function),
            zoom_channel: value.zoom_channel.unwrap_or(default.zoom_channel),
            zoom_channel_min: value.zoom_channel_min.unwrap_or(default.zoom_channel_min),
            zoom_channel_trim: value.zoom_channel_trim.unwrap_or(default.zoom_channel_trim),
            zoom_channel_max: value.zoom_channel_max.unwrap_or(default.zoom_channel_max),
            enable_focus_and_zoom_correlation: value
                .enable_focus_and_zoom_correlation
                .unwrap_or(default.enable_focus_and_zoom_correlation),
            tilt_channel: value.tilt_channel.unwrap_or(default.tilt_channel),
            tilt_channel_min: value.tilt_channel_min.unwrap_or(default.tilt_channel_min),
            tilt_channel_trim: value.tilt_channel_trim.unwrap_or(default.tilt_channel_trim),
            tilt_channel_max: value.tilt_channel_max.unwrap_or(default.tilt_channel_max),
            tilt_channel_reversed: value
                .tilt_channel_reversed
                .unwrap_or(default.tilt_channel_reversed),
            tilt_mnt_type: value.tilt_mnt_type.unwrap_or(default.tilt_mnt_type),
            tilt_mnt_pitch_min: value
                .tilt_mnt_pitch_min
                .unwrap_or(default.tilt_mnt_pitch_min),
            tilt_mnt_pitch_max: value
                .tilt_mnt_pitch_max
                .unwrap_or(default.tilt_mnt_pitch_max),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum TiltChannelFunction {
    #[default]
    /// Mount1Pitch
    MNT1 = 7,
    /// Mount2Pitch
    MNT2 = 13,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum MountType {
    Servo = 1,
    #[default]
    BrushlessPWM = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum ServoChannel {
    SERVO1 = 1,
    SERVO2 = 2,
    SERVO3 = 3,
    SERVO4 = 4,
    SERVO5 = 5,
    SERVO6 = 6,
    SERVO7 = 7,
    SERVO8 = 8,
    SERVO9 = 9,
    SERVO10 = 10,
    SERVO11 = 11,
    SERVO12 = 12,
    SERVO13 = 13,
    SERVO14 = 14,
    SERVO15 = 15,
    SERVO16 = 16,
    SERVO17 = 17,
    SERVO18 = 18,
    SERVO19 = 19,
    SERVO20 = 20,
    SERVO21 = 21,
    SERVO22 = 22,
    SERVO23 = 23,
    SERVO24 = 24,
    SERVO25 = 25,
    SERVO26 = 26,
    SERVO27 = 27,
    SERVO28 = 28,
    SERVO29 = 29,
    SERVO30 = 30,
    SERVO31 = 31,
    SERVO32 = 32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
pub enum ScriptFunction {
    #[default]
    SCRIPT1 = 94,
    SCRIPT2 = 95,
    SCRIPT3 = 96,
    SCRIPT4 = 97,
    SCRIPT5 = 98,
    SCRIPT6 = 99,
    SCRIPT7 = 100,
    SCRIPT8 = 101,
    SCRIPT9 = 102,
    SCRIPT10 = 103,
    SCRIPT11 = 104,
    SCRIPT12 = 105,
    SCRIPT13 = 106,
    SCRIPT14 = 107,
    SCRIPT15 = 108,
    SCRIPT16 = 109,
}

/// 92 is CameraFocus
pub const FOCUS_CHANNEL_FUNCTION: u8 = 92;

/// 180 is some custom value
pub const ZOOM_CHANNEL_FUNCTION: u8 = 180;

///
pub const TILT_CHANNEL_FUNCTION: TiltChannelFunction = TiltChannelFunction::MNT1;
