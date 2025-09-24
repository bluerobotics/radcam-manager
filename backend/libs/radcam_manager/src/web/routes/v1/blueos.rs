use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Debug, Serialize)]
/// https://blueos.cloud/docs/latest/development/extensions/#web-interface-http-server
pub struct ServerMetadata {
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub company: &'static str,
    pub version: &'static str,
    pub webpage: &'static str,
    pub api: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_query: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avoid_iframes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_page: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub works_in_relative_paths: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<Extras>,
}

#[derive(Debug, Serialize)]
pub struct Extras {
    pub cockpit: &'static str,
}

impl Default for ServerMetadata {
    fn default() -> Self {
        Self {
            name: "RadCam Manager",
            description: "The official management interface for RadCam",
            icon: "mdi-camera-outline",
            company: "BlueRobotics",
            version: env!("CARGO_PKG_VERSION"),
            webpage: "https://github.com/BluerRobotics/radcam-manager",
            api: "/docs",
            route: None,
            extra_query: None,
            new_page: Some(false),
            avoid_iframes: Some(false),
            works_in_relative_paths: Some(true),
            extras: Some(Extras {
                cockpit: "/cockpit_extras.json",
            }),
        }
    }
}

/// The "register_service" route is used by BlueOS extensions manager
pub async fn server_metadata() -> impl IntoResponse {
    let server_metadata = ServerMetadata::default();

    let json = serde_json::to_string_pretty(&server_metadata).unwrap();

    json.into_response()
}
