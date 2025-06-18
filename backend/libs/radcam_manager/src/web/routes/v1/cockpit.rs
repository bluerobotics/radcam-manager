use axum::response::IntoResponse;
use mcm_client::Cameras;
use radcam_commands::{
    CameraControl, protocol::display::advanced_display::AdvancedParameterSetting,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub struct CockpitExtras {
    pub target_system: String,
    pub target_cockpit_api_version: String,
    pub widgets: Vec<CockpitWidget>,
    pub actions: Vec<CockpitAction>,
}

#[derive(Debug, Serialize)]
pub struct CockpitWidget {
    pub name: String,
    pub config_iframe_url: Option<String>,
    pub iframe_url: String,
    pub iframe_icon: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct CockpitAction {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub action_type: CockpitActionType,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "config", rename_all = "kebab-case")]
pub enum CockpitActionType {
    HttpRequest(HttpRequestAction),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestAction {
    name: String,
    url: String,
    method: HttpRequestMethod,
    headers: serde_json::Value,
    url_params: serde_json::Value,
    body: String,
}

#[derive(Debug, Serialize)]
pub enum HttpRequestMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

pub async fn cockpit_extras() -> impl IntoResponse {
    let cameras = mcm_client::cameras().await;

    let cockpit_extras = CockpitExtras {
        target_system: "Cockpit".to_string(),
        target_cockpit_api_version: "1.0.0".to_string(),
        widgets: widgets(&cameras),
        actions: actions(&cameras),
    };

    let json = serde_json::to_string_pretty(&cockpit_extras).unwrap();

    json.into_response()
}

fn widgets(cameras: &Cameras) -> Vec<CockpitWidget> {
    let version = option_env!("CARGO_PKG_VERSION")
        .unwrap_or("0.0.0")
        .to_string();

    let widgets = cameras
        .iter()
        .map(|(camera_uuid, camera)| CockpitWidget {
            name: format!("RadCam ({})", camera.hostname),
            config_iframe_url: None,
            iframe_url: format!("/?uuid={camera_uuid}"),
            iframe_icon: "/assets/logo.svg".to_string(),
            version: version.clone(),
        })
        .collect::<Vec<CockpitWidget>>();

    widgets
}

fn actions(cameras: &Cameras) -> Vec<CockpitAction> {
    let actions = cameras
        .iter()
        .map(|(camera_uuid, camera)| {
            let name: String = format!("RadCam White Balance ({})", camera.hostname);

            vec![CockpitAction {
                id: format!("radcam-white-balance-{camera_uuid}"),
                name: name.clone(),
                action_type: CockpitActionType::HttpRequest(HttpRequestAction {
                    name,
                    url: "http://{{ vehicle-address }}/extensionv2/radcammanager/v1/camera/control"
                        .to_string(),
                    method: HttpRequestMethod::POST,
                    headers: json!({
                        "Content-Type": "application/json",
                    }),
                    url_params: json!({}),
                    body: json!(CameraControl {
                        camera_uuid: *camera_uuid,
                        action: radcam_commands::Action::SetImageAdjustmentEx(
                            AdvancedParameterSetting {
                                once_awb: Some(1),
                                ..Default::default()
                            }
                        ),
                    })
                    .to_string(),
                }),
            }]
        })
        .flatten()
        .collect::<Vec<CockpitAction>>();
    actions
}
