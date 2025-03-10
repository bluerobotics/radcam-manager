use axum::response::IntoResponse;
use mcm_client::Cameras;
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
    method: HttpRequestMethod,
    url: String,
    headers: serde_json::Value,
    url_params: serde_json::Value,
    body: serde_json::Value,
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

    let mut widgets = cameras
        .iter()
        .filter_map(|(camera_uuid, camera)| {
            Some(CockpitWidget {
                name: format!("RadCam ({})", camera.hostname),
                config_iframe_url: None,
                iframe_url: format!("/?uuid={camera_uuid}"),
                iframe_icon: "/assets/logo.svg".to_string(),
                version: version.clone(),
            })
        })
        .collect::<Vec<CockpitWidget>>();

    widgets.push(CockpitWidget {
        name: "RadCam (any)".to_string(),
        config_iframe_url: None,
        iframe_url: "".to_string(),
        iframe_icon: "/assets/logo.svg".to_string(),
        version: version.clone(),
    });

    widgets
}

fn actions(cameras: &Cameras) -> Vec<CockpitAction> {
    let actions = cameras
        .iter()
        .filter_map(|(camera_uuid, _camera)| {
            Some(CockpitAction {
                id: format!("radcam-white-balance-{camera_uuid}"),
                name: format!("RadCam White Balance"),
                action_type: CockpitActionType::HttpRequest(HttpRequestAction {
                    method: HttpRequestMethod::POST,
                    url: "http://{{ vehicle-address }}/extensionv2/radcam-manager/v1/camera/auto_white_balance"
                        .to_string(),
                    headers: json!({
                        "Content-Type": "application/json",
                    }),
                    url_params: json!({
                        "camera_uuid": camera_uuid,
                    }),
                    body: json!(""),
                }),
            })
        })
        .collect::<Vec<CockpitAction>>();
    actions
}
