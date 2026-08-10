use serde::{Deserialize, Serialize};
use serde_repr::*;

pub mod video_parameters;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VideoCommand {
    /// 21.1. Get Video Parameter Setting
    GetBase(video_parameters::VideoParameterSettings),
    /// 21.2. Set Video Parameter Setting
    SetBase(video_parameters::VideoParameterSettings),
}
