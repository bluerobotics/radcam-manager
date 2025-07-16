use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, TS)]
pub struct Info {
    /// Name of the program
    pub name: String,
    /// Version/tag
    pub version: String,
    /// Git SHA
    pub sha: String,
    pub build_date: String,
    /// Authors name
    pub authors: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, TS)]
pub struct ApiVideoSource {
    pub name: String,
    pub source: String,
    pub formats: Vec<Format>,
    // pub controls: Vec<Control>, # We don't need this
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
pub struct Format {
    pub encode: VideoEncodeType,
    pub sizes: Vec<Size>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "UPPERCASE")]
pub enum VideoEncodeType {
    H264,
    H265,
    Mjpg,
    Rgb,
    Unknown(String),
    Yuyv,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
pub struct Size {
    pub width: u32,
    pub height: u32,
    pub intervals: Vec<FrameInterval>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
pub struct FrameInterval {
    pub numerator: u32,
    pub denominator: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
pub struct StreamInformation {
    pub endpoints: Vec<Url>,
    pub configuration: CaptureConfiguration,
    pub extended_configuration: Option<ExtendedConfiguration>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CaptureConfiguration {
    Video(VideoCaptureConfiguration),
    Redirect(RedirectCaptureConfiguration),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
pub struct ExtendedConfiguration {
    pub thermal: bool,
    pub disable_mavlink: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
pub struct VideoCaptureConfiguration {
    pub encode: VideoEncodeType,
    pub height: u32,
    pub width: u32,
    pub frame_interval: FrameInterval,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, TS)]
pub struct RedirectCaptureConfiguration {}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub struct StreamStatus {
    pub id: uuid::Uuid,
    pub running: bool,
    pub error: Option<String>,
    pub video_and_stream: VideoAndStreamInformation,
    pub mavlink: Option<MavlinkComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub struct VideoAndStreamInformation {
    pub name: String,
    pub stream_information: StreamInformation,
    pub video_source: VideoSourceType,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub struct MavlinkComponent {
    pub system_id: u8,
    pub component_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub enum VideoSourceType {
    Gst(VideoSourceGst),
    Local(VideoSourceLocal),
    Onvif(VideoSourceOnvif),
    Redirect(VideoSourceRedirect),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub struct VideoSourceGst {
    pub name: String,
    pub source: VideoSourceGstType,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub enum VideoSourceGstType {
    // TODO: local should have a pipeline also
    Local(VideoSourceLocal),
    Fake(String),
    QR(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub struct VideoSourceLocal {
    pub name: String,
    pub device_path: String,
    #[serde(rename = "type")]
    pub typ: VideoSourceLocalType,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub enum VideoSourceLocalType {
    Unknown(String),
    Usb(String),
    LegacyRpiCam(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub struct VideoSourceRedirect {
    pub name: String,
    pub source: VideoSourceRedirectType,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub enum VideoSourceRedirectType {
    Redirect(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub struct VideoSourceOnvif {
    pub name: String,
    pub source: VideoSourceOnvifType,
    #[serde(flatten)]
    pub device_information: OnvifDeviceInformation,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
pub enum VideoSourceOnvifType {
    Onvif(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TS)]
pub struct OnvifDeviceInformation {
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub hardware_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
pub struct PostStream {
    pub name: String,
    pub source: String,
    pub stream_information: StreamInformation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
pub struct RemoveStream {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
pub struct OnvifDevice {
    pub uuid: uuid::Uuid,
    pub ip: Ipv4Addr,
    pub types: Vec<String>,
    pub hardware: Option<String>,
    pub name: Option<String>,
    pub urls: Vec<Url>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct AuthenticateOnvifDeviceRequest {
    /// Onvif Device UUID, obtained via `/onvif/devices` get request
    pub device_uuid: uuid::Uuid,
    /// Username for the Onvif Device
    pub username: String,
    /// Password for the Onvif Device
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct UnauthenticateOnvifDeviceRequest {
    /// Onvif Device UUID, obtained via `/onvif/devices` get request
    pub device_uuid: uuid::Uuid,
}
