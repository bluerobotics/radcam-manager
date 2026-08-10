use anyhow::{Context, Result, anyhow};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::sync::RwLock;
use tracing::*;

static MANAGER: OnceCell<RwLock<Manager>> = OnceCell::new();

#[derive(Debug)]
struct Manager {
    blueos_address: SocketAddr,
}

/// Constructs our manager, Should be done inside main
#[instrument(level = "debug")]
pub async fn init(blueos_address: SocketAddr) {
    if let Some(manager) = MANAGER.get() {
        manager.write().await.blueos_address = blueos_address;
        return;
    }

    MANAGER.get_or_init(|| RwLock::new(Manager { blueos_address }));
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
struct Endpoint {
    name: String,
    owner: String,
    connection_type: String,
    place: String,
    argument: u16,
    persistent: Option<bool>,
    protected: Option<bool>,
    enabled: Option<bool>,
    overwrite_settings: Option<bool>,
    __pydantic_initialised__: Option<bool>,
}

pub async fn create_mavlink_endpoint(mavlink_endpoint: &str) -> Result<()> {
    ensure_mavlink_endpoint(mavlink_endpoint).await.map(|_| ())
}

/// Make sure BlueOS still has our MAVLink endpoint.
///
/// Returns `Ok(true)` when the endpoint was created or rewritten, `Ok(false)` when
/// it was already present and correct.
pub async fn ensure_mavlink_endpoint(mavlink_endpoint: &str) -> Result<bool> {
    let blueos_address = MANAGER.get().unwrap().read().await.blueos_address;

    let desired_endpoint = {
        let (kind, address) = mavlink_endpoint
            .split_once(":")
            .context("Invalid mavlink endpoint")?;
        let (_host, port) = address
            .split_once(':')
            .context("Invalid mavlink endpoint")?;
        let port = port.parse::<u16>().context("Wrong port")?;

        let kind = match kind {
            "udpin" => "udpout",
            "udpout" => "udpin",
            "tcpin" => "tcpout",
            "tcpout" => "tcpin",
            _ => return Err(anyhow!("Unsupported endpoint kind: {kind:?}")),
        };

        Endpoint {
            name: "4K Cam Manager".to_string(),
            owner: "br4kcam-manager".to_string(),
            connection_type: kind.to_string(),
            place: "0.0.0.0".to_string(),
            argument: port,
            persistent: Some(true),
            protected: Some(false),
            enabled: Some(true),
            overwrite_settings: Some(false),
            __pydantic_initialised__: Some(true),
        }
    };

    let current_endpoints: Vec<Endpoint> =
        web_client::get(&blueos_address, "ardupilot-manager/v1.0/endpoints/", (), ())
            .await
            .context("Failed getting MAVLink endpoints from BlueOS")?;

    // Match by owner+port (not display name) so renaming the endpoint does not create a duplicate.
    if let Some(existing_endpoint) = current_endpoints.iter().find(|current| {
        current.owner == desired_endpoint.owner
            && current.argument == desired_endpoint.argument
            && current.connection_type == desired_endpoint.connection_type
            && current.place == desired_endpoint.place
    }) {
        if desired_endpoint.eq(existing_endpoint) {
            debug!("MAVLink endpoint already present");
            return Ok(false);
        }

        info!("MAVLink endpoint exists but needs to be reconfigured.");

        web_client::put::<(), _, _>(
            &blueos_address,
            "ardupilot-manager/v1.0/endpoints/",
            vec![desired_endpoint],
            (),
        )
        .await
        .context("Failed to create new MAVLink endpoint")?;
        return Ok(true);
    }

    info!("MAVLink endpoint not present, creating it...");

    web_client::post::<(), _, _>(
        &blueos_address,
        "ardupilot-manager/v1.0/endpoints/",
        vec![desired_endpoint],
        (),
    )
    .await
    .context("Failed to create new MAVLink endpoint")?;
    Ok(true)
}

pub async fn reboot_autopilot() -> Result<()> {
    let blueos_address = MANAGER.get().unwrap().read().await.blueos_address;

    web_client::post(&blueos_address, "ardupilot-manager/v1.0/restart", (), ()).await
}
