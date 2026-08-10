use serde::Deserialize;

pub mod display;
pub mod video;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Response {
    pub code: i32,
    pub device_mac: String,
    pub device_id: String,
    pub log: String,
    pub device_ip: String,
    pub sign_tby: String,
}
