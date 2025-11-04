use std::{future::Future, pin::Pin};

use anyhow::{Context, Result};
use axum::{Json, http::StatusCode, response::IntoResponse};
use mcm_client::{Camera, get_camera};
use protocol::{
    display::{advanced_display::AdvancedParameterSetting, base_display::BaseParameterSetting},
    video::video_parameters::VideoParameterSettings,
};
use serde::{Deserialize, Serialize};
use tracing::*;
use ts_rs::TS;
use url::Url;
use uuid::Uuid;
use web_client::send_request;

pub mod protocol;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
// #[tsync] // FIXME: Disabled for now, see https://github.com/Wulf/tsync/issues/58
pub struct CameraControl {
    #[ts(as = "String")]
    pub camera_uuid: Uuid,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "action", content = "json")]
// #[tsync] // FIXME: Disabled for now, see https://github.com/Wulf/tsync/issues/58
pub enum Action {
    #[serde(rename = "getSysConfig")]
    GetSysConfig,
    #[serde(rename = "getImageAdjustment")]
    GetImageAdjustment,
    #[serde(rename = "getImageAdjustmentEx")]
    GetImageAdjustmentEx,
    #[serde(rename = "getVencConf")]
    GetVideoParameterSettings(VideoParameterSettings),
    #[serde(rename = "setImageAdjustment")]
    SetImageAdjustment(BaseParameterSetting),
    #[serde(rename = "setImageAdjustmentEx")]
    SetImageAdjustmentEx(AdvancedParameterSetting),
    #[serde(rename = "setVencConf")]
    SetVideoParameterSettings(VideoParameterSettings),
    #[serde(rename = "restart")]
    Restart,
    /// Important: This is a wrapper, not part of the camera protocol
    #[serde(rename = "setRecommendedSettings")]
    SetRecommendedSettings,
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[instrument(level = "debug")]
fn control_inner(
    camera_control: Json<CameraControl>,
) -> Pin<Box<dyn Future<Output = Result<serde_json::Value>> + Send>> {
    Box::pin(async move {
        debug!("Got control query: {camera_control:#?}");

        // Special case for command wrappers:
        match &camera_control.action {
            Action::SetRecommendedSettings => {
                return apply_recommended_settings(camera_control.camera_uuid).await;
            }
            _ => (),
        }

        let action_value = serde_json::to_value(&camera_control.action).unwrap();
        let action_map = action_value.as_object().unwrap();
        let payload = action_map
            .get("json")
            .map(|value| value.to_string())
            .unwrap_or_default();

        let url = get_camera_api_url(&camera_control.0).await?;

        debug!("URL: {url}, json: {payload}");

        let res: serde_json::Value =
            send_request(reqwest::Client::new().post(url), payload).await?;

        debug!("Answer from the camera: {res:#?}");

        let res = match &camera_control.action {
            Action::SetImageAdjustment(_) => {
                let mut camera_control = camera_control.0.clone();
                camera_control.action = Action::GetImageAdjustment;

                control_inner(Json(camera_control)).await
            }
            Action::SetImageAdjustmentEx(_) => {
                let mut camera_control = camera_control.0.clone();
                camera_control.action = Action::GetImageAdjustmentEx;

                control_inner(Json(camera_control)).await
            }
            Action::SetVideoParameterSettings(video_parameters_settings) => {
                let mut camera_control = camera_control.0.clone();
                camera_control.action = Action::GetVideoParameterSettings(VideoParameterSettings {
                    channel: video_parameters_settings.channel.clone(),
                    ..Default::default()
                });

                control_inner(Json(camera_control)).await
            }
            Action::Restart => {
                let mut camera_control = camera_control.0.clone();
                camera_control.action = Action::GetSysConfig;
                let value = Json(camera_control);

                tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

                let mut tries = 20;
                while tries > 0 {
                    debug!("Waiting for camera to restart...");

                    tries -= 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                    if control_inner(value.clone()).await.is_ok() {
                        break;
                    }
                }

                Ok(res)
            }
            _ => Ok(res),
        }?;

        debug!("res.to_string(): {}", res.to_string());

        Ok(res)
    })
}

#[instrument(level = "debug")]
pub async fn control(camera_control: Json<CameraControl>) -> impl IntoResponse {
    let res = match control_inner(camera_control).await {
        Ok(res) => res,
        Err(error) => {
            warn!("res from send_request: {error:#?}");
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error:?}")).into_response();
        }
    };

    (StatusCode::OK, res.to_string()).into_response()
}

#[instrument(level = "debug")]
pub async fn list() -> impl IntoResponse {
    let cameras = mcm_client::cameras().await;

    let json = match serde_json::to_string(&cameras) {
        Ok(json) => json,
        Err(error) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("{error:?}")).into_response();
        }
    };

    json.into_response()
}

#[instrument(level = "debug")]
pub async fn apply_recommended_settings(camera_uuid: Uuid) -> Result<serde_json::Value> {
    let mut errors = vec![];

    info!("Applying recommended settings to camera {camera_uuid:?}");

    // Set main channel to 4k@30fps, and unused channels to 480p@5fps
    {
        use protocol::video::video_parameters::*;

        let channel_configs = vec![
            CameraControl {
                camera_uuid,
                action: Action::SetVideoParameterSettings(VideoParameterSettings {
                    channel: Some(VideoChannelValue::MainStream),
                    encode_profile: Some(VideoEncodingProfileValue::HighProfile),
                    encode_type: Some(VideoEncodeTypeValue::H264),
                    pic_width: Some(3840),
                    pic_height: Some(2160),
                    rc_mode: Some(VideoRcModeValue::ConstantBitRate),
                    bitrate: Some(16384),
                    frame_rate: Some(30),
                    gop: Some(60),
                    ..Default::default()
                }),
            },
            CameraControl {
                camera_uuid,
                action: Action::SetVideoParameterSettings(VideoParameterSettings {
                    channel: Some(VideoChannelValue::AuxiliaryStream),
                    encode_profile: Some(VideoEncodingProfileValue::Baseline),
                    encode_type: Some(VideoEncodeTypeValue::H264),
                    pic_width: Some(640),
                    pic_height: Some(480),
                    rc_mode: Some(VideoRcModeValue::ConstantBitRate),
                    bitrate: Some(32),
                    frame_rate: Some(5),
                    gop: Some(60),
                    ..Default::default()
                }),
            },
            CameraControl {
                camera_uuid,
                action: Action::SetVideoParameterSettings(VideoParameterSettings {
                    channel: Some(VideoChannelValue::ThirdStream),
                    encode_profile: Some(VideoEncodingProfileValue::Baseline),
                    encode_type: Some(VideoEncodeTypeValue::H264),
                    pic_width: Some(640),
                    pic_height: Some(480),
                    rc_mode: Some(VideoRcModeValue::ConstantBitRate),
                    bitrate: Some(32),
                    frame_rate: Some(5),
                    gop: Some(60),
                    ..Default::default()
                }),
            },
        ];

        for camera_control in channel_configs {
            if let Err(error) = control_inner(Json(camera_control)).await {
                let message = format!(
                    "Failed applying recommended settings for VideoParameterSettings: {error:?}"
                );
                error!(message);
                errors.push(message);
            }
        }
    }

    {
        use protocol::display::base_display::*;

        let action = Action::SetImageAdjustment(BaseParameterSetting {
            set_default: Some(1),
            ..Default::default()
        });

        let camera_control = CameraControl {
            camera_uuid,
            action,
        };

        if let Err(error) = control_inner(Json(camera_control)).await {
            let message =
                format!("Failed applying recommended settings for BaseParameterSetting: {error:?}");
            error!(message);
            errors.push(message);
        }
    }

    {
        use protocol::display::advanced_display::*;

        let action = Action::SetImageAdjustmentEx(AdvancedParameterSetting {
            set_default: Some(1),
            ..Default::default()
        });

        let camera_control = CameraControl {
            camera_uuid,
            action,
        };

        if let Err(error) = control_inner(Json(camera_control)).await {
            let message = format!(
                "Failed applying recommended settings for AdvancedParameterSetting: {error:?}"
            );
            error!(message);
            errors.push(message);
        }
    }

    match errors.len() {
        0 => Ok(serde_json::Value::Null),
        1 => Err(anyhow::anyhow!("{}", errors[0])),
        _ => Err(anyhow::anyhow!("Multiple errors happened: {errors:?}")),
    }
}

#[instrument(level = "debug")]
async fn get_camera_api_url(camera_control: &CameraControl) -> Result<Url> {
    let camera_uuid = camera_control.camera_uuid;
    let action_value = serde_json::to_value(&camera_control.action).unwrap();
    let action_map = action_value.as_object().unwrap();
    let action = action_map.get("action").unwrap().as_str().unwrap();

    let Camera {
        hostname,
        credentials,
        ..
    } = get_camera(&camera_uuid).await.context("Camera not found")?;

    let port = 80;

    match credentials {
        // NOTE: This part of the protocol is not working: only the get requests using cgi_action parameter are working.
        // I'm leaving it here in case they fix it:
        // Some(Credentials { username, password }) => {
        //     format!(
        //         "http://{hostname}:{port}/action/cgi_action?user={username}&pwd={password}&action={action}", password=hash_password(&password)
        //     )
        // }
        // None => format!("http://{hostname}:{port}/action/{action}"),
        // So for now we are just using the protocol without the authentication:
        _ => format!("http://{hostname}:{port}/action/{action}"),
    }
    .parse()
    .context("Invalid URl")
}

#[instrument(level = "debug")]
pub fn hash_password(password: &str) -> String {
    use md5::{Digest, Md5};

    let mut hasher = Md5::new();
    hasher.update(password.as_bytes());

    base16ct::lower::encode_string(&hasher.finalize())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use utils::deserialize;

    use super::CameraControl;

    #[test]
    fn action_serde_test() {
        let payload = json!({
            "camera_uuid": "bc071801-c50f-8301-ac36-bc071801c50f",
            "action": "setImageAdjustmentEx",
            "json": {
                "onceAWB": 1
            }
        })
        .to_string();

        let camera_control = deserialize::<CameraControl>(&payload).unwrap();

        let serialized_action = serde_json::to_string(&camera_control.action).unwrap();

        let expected_action = json!({
            "action": "setImageAdjustmentEx",
            "json": {
                "onceAWB": 1
            }
        })
        .to_string();

        assert_eq!(expected_action, serialized_action);
    }
}
