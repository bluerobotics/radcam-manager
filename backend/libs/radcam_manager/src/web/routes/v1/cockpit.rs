use axum::response::IntoResponse;
use mcm_client::Cameras;
use radcam_commands::{
    CameraControl, protocol::display::advanced_display::AdvancedParameterSetting,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CockpitExtras {
    pub target_system: String,
    pub target_cockpit_api_version: String,
    pub widgets: Vec<CockpitIframeWidget>,
    pub actions: Vec<CockpitAction>,
    pub joystick_suggestions: Vec<JoystickMapSuggestion>,
}

/// Widget configuration object as received from BlueOS or another external source
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CockpitIframeWidget {
    /// Name of the widget, this is displayed on edit mode widget browser
    pub name: String,
    /// The URL at which the widget is located. Whether this is a relative or absolute path depends on the use_vehicle_address_as_base_url field
    pub iframe_url: String,
    /// The icon of the widget, this is displayed on the widget browser
    pub iframe_icon: String,
    /// The name of the collapsed container, this is displayed on the widget browser
    pub collapsible_container_name: String,
    /// Version of the widget (optional)
    pub version: Option<String>,
    /// Whether the widget should start collapsed (optional)
    pub start_collapsed: bool,
    /// Whether to use vehicle address as base URL for the widget (optional)
    pub use_vehicle_address_as_base_url: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CockpitAction {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub action_type: CockpitActionType,
    /// Version of this Action
    pub version: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", content = "config", rename_all = "kebab-case")]
pub enum CockpitActionType {
    HttpRequest(HttpRequestAction),
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestAction {
    name: String,
    url: String,
    method: HttpRequestMethod,
    headers: serde_json::Value,
    url_params: serde_json::Value,
    body: String,
}

#[derive(Debug, Serialize, Clone, Copy)]
pub enum HttpRequestMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

/// Joystick map suggestion from BlueOS extensions
/// Example of use: https://github.com/rafaellehmkuhl/MockBlueOsExtension/blob/913eb0a978159bdb2c4e4b610044f1c8082755ab/src/backend/main.py#L147
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JoystickMapSuggestion {
    /// ID for this suggestion
    pub id: String,
    /// Name for this suggestion
    pub name: String,
    /// List of the button mapping suggestions
    pub button_mapping_suggestions: Vec<ButtonMappingSuggestion>,
    /// Version of this Joystick Map Suggestion
    pub version: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ButtonMappingSuggestion {
    /// ID for this button suggestion
    pub id: String,
    /// Protocol that holds the action
    pub action_protocol: JoystickProtocol,
    /// Human-readable name of the action to be mapped
    pub action_name: String,
    /// Unique identifier for the action to be mapped
    pub action_id: String,
    /// The button number (in Cockpit standard mapping) to map the action to
    pub button: u32,
    /// The modifier key for this suggestion (regular or shift)
    pub modifier_key: CockpitModifierKeyOption,
    /// Optional description of what the action does
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Clone, Copy)]
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

#[derive(Debug, Serialize, Clone, Copy)]
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

fn widgets(cameras: &Cameras) -> Vec<CockpitIframeWidget> {
    cameras
        .iter()
        .map(|(camera_uuid, camera)| CockpitIframeWidget {
            name: format!("RadCam ({})", camera.hostname),
            iframe_url: format!("/#/?uuid={camera_uuid}&cockpit_mode=true"),
            iframe_icon: "/assets/logo.svg".to_string(),
            collapsible_container_name: format!("RadCam ({})", camera.hostname),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            start_collapsed: true,
            use_vehicle_address_as_base_url: true,
        })
        .collect()
}

fn actions(cameras: &Cameras) -> Vec<CockpitAction> {
    cameras
        .iter()
        .flat_map(|(camera_uuid, camera)| {
            let name: String = format!("RadCam One-Push White Balance ({})", camera.hostname);

            vec![CockpitAction {
                id: format!("radcam_white_balance_{camera_uuid}"),
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
                version: env!("CARGO_PKG_VERSION").to_string(), // TODO: freeze this once we settle with a button layout
            }]
        })
        .collect()
}

fn joystick_suggestions(cameras: &Cameras) -> Vec<JoystickMapSuggestion> {
    let mappings = vec![
        // === Regular modifier ===
        ButtonMappingSuggestion {
            id: "shift".to_string(),
            action_protocol: JoystickProtocol::CockpitModifierKey,
            action_name: "Shift".to_string(),
            action_id: "shift".to_string(),
            button: 0,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Enable shift modifier".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Mode-manual".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Mode manual".to_string(),
            action_id: "Mode manual".to_string(),
            button: 1,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Switch to manual flight mode".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Mode-depth-hold".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Mode depth hold".to_string(),
            action_id: "Mode depth hold".to_string(),
            button: 2,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Switch to depth hold mode".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Mode-stabilize".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Mode stabilize".to_string(),
            action_id: "Mode stabilize".to_string(),
            button: 3,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Switch to stabilize mode".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Mount-tilt-down".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Camera Mount Tilt Down".to_string(),
            action_id: "Mount tilt down".to_string(),
            button: 4,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Move camera mount down".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Mount-tilt-up".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Camera Mount Tilt Up".to_string(),
            action_id: "Mount tilt up".to_string(),
            button: 5,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Move camera mount up".to_string()),
        },
        ButtonMappingSuggestion {
            id: "camera-focus-decrease".to_string(),
            action_protocol: JoystickProtocol::DataLakeVariable,
            action_name: "Camera focus decrease".to_string(),
            action_id: "camera-focus-decrease".to_string(),
            button: 6,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Decrease camera focus distance".to_string()),
        },
        ButtonMappingSuggestion {
            id: "camera-focus-increase".to_string(),
            action_protocol: JoystickProtocol::DataLakeVariable,
            action_name: "Camera focus increase".to_string(),
            action_id: "camera-focus-increase".to_string(),
            button: 7,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Increase camera focus distance".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Disarm".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Disarm".to_string(),
            action_id: "Disarm".to_string(),
            button: 8,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Disarm vehicle".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Arm".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Arm".to_string(),
            action_id: "Arm".to_string(),
            button: 9,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Arm vehicle".to_string()),
        },
        ButtonMappingSuggestion {
            id: "toggle_recording_all_streams".to_string(),
            action_protocol: JoystickProtocol::CockpitAction,
            action_name: "Toggle recording all streams".to_string(),
            action_id: "toggle_recording_all_streams".to_string(),
            button: 11,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Toggle recording all streams".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Gain-inc".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Gain inc".to_string(),
            action_id: "Gain inc".to_string(),
            button: 12,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Increase gain".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Gain-dec".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Gain dec".to_string(),
            action_id: "Gain dec".to_string(),
            button: 13,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Decrease gain".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Lights1-dimmer".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Lights1 dimmer".to_string(),
            action_id: "Lights1 dimmer".to_string(),
            button: 14,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Dim lights".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Lights1-brighter".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Lights1 brighter".to_string(),
            action_id: "Lights1 brighter".to_string(),
            button: 15,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Brighten lights".to_string()),
        },
        ButtonMappingSuggestion {
            id: "toggle_bottom_bar".to_string(),
            action_protocol: JoystickProtocol::CockpitAction,
            action_name: "Toggle bottom bar".to_string(),
            action_id: "toggle_bottom_bar".to_string(),
            button: 16,
            modifier_key: CockpitModifierKeyOption::Regular,
            description: Some("Toggle bottom UI bar".to_string()),
        },
        // === Shift modifier ===
        ButtonMappingSuggestion {
            id: "take_snapshot".to_string(),
            action_protocol: JoystickProtocol::CockpitAction,
            action_name: "Take a Snapshot".to_string(),
            action_id: "take_snapshot".to_string(),
            button: 2,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Take a snapshot".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Mode-acro".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Mode acro".to_string(),
            action_id: "Mode acro".to_string(),
            button: 3,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Switch to acro flight mode".to_string()),
        },
        ButtonMappingSuggestion {
            id: "camera-zoom-decrease".to_string(),
            action_protocol: JoystickProtocol::DataLakeVariable,
            action_name: "Camera zoom decrease".to_string(),
            action_id: "camera-zoom-decrease".to_string(),
            button: 6,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Decrease camera zoom level".to_string()),
        },
        ButtonMappingSuggestion {
            id: "camera-zoom-increase".to_string(),
            action_protocol: JoystickProtocol::DataLakeVariable,
            action_name: "Camera zoom increase".to_string(),
            action_id: "camera-zoom-increase".to_string(),
            button: 7,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Increase camera zoom level".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Trim-pitch-inc".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Trim pitch inc".to_string(),
            action_id: "Trim pitch inc".to_string(),
            button: 12,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Increase pitch trim".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Trim-pitch-dec".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Trim pitch dec".to_string(),
            action_id: "Trim pitch dec".to_string(),
            button: 13,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Decrease pitch trim".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Trim-roll-dec".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Trim roll dec".to_string(),
            action_id: "Trim roll dec".to_string(),
            button: 14,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Decrease roll trim".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Trim-roll-inc".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Trim roll inc".to_string(),
            action_id: "Trim roll inc".to_string(),
            button: 15,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Increase roll trim".to_string()),
        },
        ButtonMappingSuggestion {
            id: "toggle_top_bar".to_string(),
            action_protocol: JoystickProtocol::CockpitAction,
            action_name: "Toggle top bar".to_string(),
            action_id: "toggle_top_bar".to_string(),
            button: 16,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Toggle top UI bar".to_string()),
        },
    ];

    let mut mappings_with_gripper = mappings.clone();
    mappings_with_gripper.extend_from_slice(&[
        ButtonMappingSuggestion {
            id: "Actuator-1-max-momentary".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Open Gripper".to_string(),
            action_id: "Actuator 1 max momentary".to_string(),
            button: 4,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Open gripper".to_string()),
        },
        ButtonMappingSuggestion {
            id: "Actuator-1-min-momentary".to_string(),
            action_protocol: JoystickProtocol::MavlinkManualControl,
            action_name: "Close Gripper".to_string(),
            action_id: "Actuator 1 min momentary".to_string(),
            button: 5,
            modifier_key: CockpitModifierKeyOption::Shift,
            description: Some("Close gripper".to_string()),
        },
    ]);

    let mut suggestions = vec![
        JoystickMapSuggestion {
            id: "radcam-only".to_string(),
            name: "RadCam only".to_string(),
            button_mapping_suggestions: mappings,
            version: env!("CARGO_PKG_VERSION").to_string(), // TODO: freeze this once we settle with a button layout
        },
        JoystickMapSuggestion {
            id: "radcam-with-gripper".to_string(),
            name: "RadCam with gripper".to_string(),
            button_mapping_suggestions: mappings_with_gripper,
            version: env!("CARGO_PKG_VERSION").to_string(), // TODO: freeze this once we settle with a button layout
        },
    ];

    for (camera_uuid, _camera) in cameras {
        suggestions.push(JoystickMapSuggestion {
            id: format!("RadCam-{camera_uuid}"),
            name: format!("RadCam (camera {camera_uuid})"),
            button_mapping_suggestions: vec![ButtonMappingSuggestion {
                id: format!("radcam_white_balance_{camera_uuid}"),
                action_protocol: JoystickProtocol::CockpitAction,
                action_name: format!("RadCam One-Push White Balance ({})", _camera.hostname),
                action_id: format!("radcam_white_balance_{camera_uuid}"),
                button: 10,
                modifier_key: CockpitModifierKeyOption::Regular,
                description: Some("Run One-Push White Balance once".to_string()),
            }],
            version: env!("CARGO_PKG_VERSION").to_string(), // TODO: freeze this once we settle with a button layout
        });
    }

    suggestions
}
