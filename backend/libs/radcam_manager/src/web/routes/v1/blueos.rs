use axum::response::IntoResponse;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ServerMetadata {
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    pub company: &'static str,
    pub version: &'static str,
    pub new_page: bool,
    pub webpage: &'static str,
    pub api: &'static str,
    pub extras: Extras,
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
            version: option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"),
            new_page: false,
            webpage: "https://github.com/BluerRobotics/radcam-manager",
            api: "/docs",
            extras: Extras {
                cockpit: "/cockpit_extras.json",
            },
        }
    }
}

/// The "register_service" route is used by BlueOS extensions manager
pub async fn server_metadata() -> impl IntoResponse {
    let server_metadata = ServerMetadata::default();

    let json = serde_json::to_string_pretty(&server_metadata).unwrap();

    json.into_response()
}
