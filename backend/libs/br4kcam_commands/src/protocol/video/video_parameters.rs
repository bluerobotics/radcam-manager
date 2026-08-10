use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;
use tsync::tsync;

use super::*;

#[skip_serializing_none]
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[tsync]
pub struct VideoParameterSettings {
    /// Video stream flow channel.
    pub channel: Option<VideoChannelValue>,
    /// Encoding scheme.
    pub encode_profile: Option<VideoEncodingProfileValue>,
    /// Video Coding.
    pub encode_type: Option<VideoEncodeTypeValue>,
    /// Supported video pixel resolution.
    pub pixel_list: Option<Vec<VideoResolutionValue>>,
    /// Video width resolution, from pixel_list.
    pub pic_width: Option<u16>,
    /// Video height resolution, from pixel_list.
    pub pic_height: Option<u16>,
    /// Bitrate type.
    pub rc_mode: Option<VideoRcModeValue>,
    /// Video bitrate.
    pub bitrate: Option<u16>,
    /// Supported maximum vidoe frame rate.
    pub max_framerate: Option<u16>,
    /// Video frame rate.
    pub frame_rate: Option<u16>,
    /// The I-Frame interval.
    pub gop: Option<u16>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[tsync]
pub struct VideoResolutionValue {
    width: u16,
    height: u16,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
#[tsync]
pub enum VideoChannelValue {
    #[default]
    MainStream = 0,
    AuxiliaryStream = 1,
    ThirdStream = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
#[tsync]
pub enum VideoEncodingProfileValue {
    #[default]
    Baseline = 0,
    MainProfile = 1,
    HighProfile = 2,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
#[tsync]
pub enum VideoEncodeTypeValue {
    #[default]
    H264 = 1,
    H265 = 5,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize_repr, Deserialize_repr, TS)]
#[repr(u8)]
#[tsync]
pub enum VideoRcModeValue {
    #[default]
    VariableBitRate = 0,
    ConstantBitRate = 1,
}

#[cfg(test)]
mod tes {

    use serde_json::json;

    use utils::deserialize;

    use super::*;

    #[test]
    fn setting_some_parameters() {
        let json = json!({
            "bitrate": 6144,
            "channel": 0,
            "encode_profile": 1,
            "encode_type": 1,
            "frame_rate": 25,
            "gop": 50,
            "max_framerate": 25,
            "pic_height": 1440,
            "pic_width": 2560,
            "rc_mode": 0,
            "pixel_list": [
                {
                    "width": 2560,
                    "height": 1440,
                },
                {
                    "width": 1920,
                    "height": 1080,
                }
            ],
        })
        .to_string();

        let params = deserialize::<VideoParameterSettings>(&json).expect("Failed deserializing");

        let expected_params = VideoParameterSettings {
            bitrate: Some(6144),
            channel: Some(VideoChannelValue::MainStream),
            encode_profile: Some(VideoEncodingProfileValue::MainProfile),
            encode_type: Some(VideoEncodeTypeValue::H264),
            frame_rate: Some(25),
            gop: Some(50),
            max_framerate: Some(25),
            pic_height: Some(1440),
            pic_width: Some(2560),
            rc_mode: Some(VideoRcModeValue::VariableBitRate),
            pixel_list: Some(vec![
                VideoResolutionValue {
                    width: 2560,
                    height: 1440,
                },
                VideoResolutionValue {
                    width: 1920,
                    height: 1080,
                },
            ]),
        };

        assert_eq!(expected_params, params);
    }
}
