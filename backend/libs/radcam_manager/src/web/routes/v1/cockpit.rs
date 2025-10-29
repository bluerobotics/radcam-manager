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
    pub joystick_suggestions: Vec<JoystickMapSuggestion>,
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

/// Joystick map suggestion from BlueOS extensions
/// Example of use: https://github.com/rafaellehmkuhl/MockBlueOsExtension/blob/913eb0a978159bdb2c4e4b610044f1c8082755ab/src/backend/main.py#L147
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoystickMapSuggestion {
    /// Unique identifier for this suggestion
    pub id: String,
    /// Protocol that holds the action
    pub protocol: JoystickProtocol,
    /// Human-readable name of the action to be mapped
    pub action_name: String,
    /// Unique identifier for the action to be mapped
    pub action_id: String,
    /// The button number (in Cockpit standard mapping) to map the action to
    pub button: u32,
    /// The modifier key for this suggestion (regular or shift)
    pub modifier: CockpitModifierKeyOption,
    /// Optional description of what the action does
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
/// Available joystick protocols.
/// Each protocol is expected to have it's own way of doing thing, including mapping, limiting, communicating, etc.
pub enum JoystickProtocol {
    CockpitModifierKey,
    MavlinkManualControl,
    CockpitAction,
    DataLakeVariable,
    Other,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
/// Modifier keys
pub enum CockpitModifierKeyOption {
    Regular,
    Shift,
}

pub async fn cockpit_extras() -> impl IntoResponse {
    let cameras = mcm_client::cameras().await;

    let cockpit_extras = CockpitExtras {
        target_system: "Cockpit".to_string(),
        target_cockpit_api_version: "1.0.0".to_string(),
        widgets: widgets(&cameras),
        actions: actions(&cameras),
        joystick_suggestions: joystick_suggestions(&cameras),
    };

    let json = serde_json::to_string_pretty(&cockpit_extras).unwrap();

    json.into_response()
}

fn widgets(cameras: &Cameras) -> Vec<CockpitWidget> {
    cameras
        .iter()
        .map(|(camera_uuid, camera)| CockpitWidget {
            name: format!("RadCam ({})", camera.hostname),
            config_iframe_url: None,
            iframe_url: format!("#/?uuid={camera_uuid}&cockpit_mode=true"),
            iframe_icon: "/assets/logo.svg".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
        .collect()
}

fn actions(cameras: &Cameras) -> Vec<CockpitAction> {
    cameras
        .iter()
        .flat_map(|(camera_uuid, camera)| {
            let name: String = format!("RadCam One-Push White Balance ({})", camera.hostname);

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
        .collect()
}

fn joystick_suggestions(_cameras: &Cameras) -> Vec<JoystickMapSuggestion> {
    let id = "RadCam Manager";
    vec![
        JoystickMapSuggestion {
            id: id.to_string(),
            protocol: JoystickProtocol::DataLakeVariable,
            action_name: "Camera focus decrease".to_string(),
            action_id: "camera-focus-decrease".to_string(),
            button: 6,
            modifier: CockpitModifierKeyOption::Regular,
            description: Some("Decrease camera focus distance".to_string()),
        },
        JoystickMapSuggestion {
            id: id.to_string(),
            protocol: JoystickProtocol::DataLakeVariable,
            action_name: "Camera focus increase".to_string(),
            action_id: "camera-focus-increase".to_string(),
            button: 7,
            modifier: CockpitModifierKeyOption::Regular,
            description: Some("Increase camera focus distance".to_string()),
        },
        JoystickMapSuggestion {
            id: id.to_string(),
            protocol: JoystickProtocol::DataLakeVariable,
            action_name: "Camera zoom decrease".to_string(),
            action_id: "camera-zoom-decrease".to_string(),
            button: 6,
            modifier: CockpitModifierKeyOption::Shift,
            description: Some("Decrease camera zoom level".to_string()),
        },
        JoystickMapSuggestion {
            id: id.to_string(),
            protocol: JoystickProtocol::DataLakeVariable,
            action_name: "Camera zoom increase".to_string(),
            action_id: "camera-zoom-increase".to_string(),
            button: 7,
            modifier: CockpitModifierKeyOption::Shift,
            description: Some("Increase camera zoom level".to_string()),
        },
    ]
}
