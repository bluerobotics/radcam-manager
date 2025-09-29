use std::net::{Ipv4Addr, SocketAddr};

use anyhow::{Context, Result};
use indexmap::IndexMap;
use mcm_client::MCMClient;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use tokio::{sync::RwLock, task::JoinHandle};
use tracing::*;
use ts_rs::TS;
use url::Url;
use uuid::Uuid;

// note: keep this private to isolate MCM API from the rest of the code
pub(crate) mod mcm_client;
pub mod mcm_types;

static MANAGER: OnceCell<RwLock<Manager>> = OnceCell::new();

#[derive(Debug)]
struct Manager {
    cameras: Cameras,
    _authentication_task_handler: JoinHandle<()>,
    _start_radcams_task_handler: JoinHandle<()>,
}

pub type Cameras = IndexMap<Uuid, Camera>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct Camera {
    pub uuid: Uuid,
    pub hostname: Ipv4Addr,
    pub credentials: Option<Credentials>,
    pub streams: Streams,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

pub type Streams = IndexMap<Uuid, Stream>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, TS)]
pub struct Stream {
    name: String,
    source_endpoint: Url,
    stream_endpoints: Vec<Url>,
}

/// Constructs our manager, Should be done inside main
#[instrument(level = "debug")]
pub async fn init(mcm_address: SocketAddr) {
    let cameras = IndexMap::new();
    let _authentication_task_handler =
        tokio::spawn(async move { authenticate_radcams(&mcm_address).await });
    let _start_radcams_task_handler =
        tokio::spawn(async move { start_radcams_streams(&mcm_address).await });

    MANAGER.get_or_init(|| {
        RwLock::new(Manager {
            cameras,
            _authentication_task_handler,
            _start_radcams_task_handler,
        })
    });
}

impl Drop for Manager {
    #[instrument(level = "debug")]
    fn drop(&mut self) {
        debug!("Finishing tasks...");

        self._authentication_task_handler.abort();
        self._start_radcams_task_handler.abort();
    }
}

#[instrument(level = "debug")]
async fn authenticate_radcams(mcm_address: &SocketAddr) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        clear_cameras().await;

        let mcm = match MCMClient::try_new(mcm_address).await {
            Ok(mcm) => mcm,
            Err(error) => {
                debug!("Failed to create MCM client: {error:?}");
                continue;
            }
        };

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let radcams = match mcm.get_radcams().await {
                Ok(radcams) => radcams,
                Err(error) => {
                    debug!("Failed to create MCM client: {error:?}");
                    break;
                }
            };

            let known_cameras = cameras().await;

            for camera in &radcams {
                if let Some(known_camera) = known_cameras.get(&camera.uuid) {
                    if known_camera != camera
                        && let Err(error) = add_camera(camera).await
                    {
                        debug!("Failed updating camera {camera:?}: {error:?}");
                        continue;
                    }

                    break;
                }

                debug!("New RadCam found: {camera:?}");

                if let Err(error) = mcm.authenticate(camera).await {
                    debug!("Failed authenticating onvif camera {camera:?}: {error:?}");
                    continue;
                }

                if let Err(error) = add_camera(camera).await {
                    debug!("Failed adding camera {camera:?}: {error:?}");
                    continue;
                }

                debug!("New RadCam added: {camera:?}");
            }
        }
    }
}

#[instrument(level = "debug")]
async fn start_radcams_streams(mcm_address: &SocketAddr) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let mcm = match MCMClient::try_new(mcm_address).await {
            Ok(mcm) => mcm,
            Err(error) => {
                debug!("Failed to create MCM client: {error:?}");
                continue;
            }
        };

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let existing_radcam_streams = match mcm.get_radcam_streams().await {
                Ok(streams) => streams,
                Err(error) => {
                    debug!("Failed to get radcam streams: {error:?}");
                    continue;
                }
            };

            let available_radcam_sources = match mcm.get_radcam_video_sources().await {
                Ok(sources) => sources,
                Err(error) => {
                    debug!("Failed to get radcam video sources: {error:?}");
                    continue;
                }
            };

            for source in available_radcam_sources {
                if !source.source.ends_with("stream_0") {
                    continue; // We only want the main stream
                }

                if existing_radcam_streams.iter().any(|stream| {
                    // Note: Here we are ignoring any authentication so we avoid duplicated streams

                    let mut existing_source = stream.source_endpoint.clone();
                    existing_source.set_password(None).unwrap();
                    existing_source.set_username("").unwrap();

                    let mut available_source: Url = source.source.clone().parse().unwrap();
                    available_source.set_password(None).unwrap();
                    available_source.set_username("").unwrap();

                    existing_source.eq(&available_source)
                }) {
                    continue;
                }

                if let Err(error) = mcm.create_stream(source).await {
                    warn!("Failed creating stream: {error:?}");
                    continue;
                }
            }
        }
    }
}

#[instrument(level = "debug")]
pub async fn cameras() -> Cameras {
    MANAGER.get().unwrap().read().await.cameras.clone()
}

#[instrument(level = "debug")]
pub async fn add_camera(camera: &Camera) -> Result<()> {
    let mut lock = MANAGER.get().unwrap().write().await;

    if let Some(old_camera) = lock.cameras.insert(camera.uuid, camera.clone()) {
        debug!("Camera updated: old: {old_camera:?}");
    }

    Ok(())
}

#[instrument(level = "debug")]
pub async fn get_camera(uuid: &Uuid) -> Option<Camera> {
    let lock = MANAGER.get().unwrap().read().await;

    lock.cameras.get(uuid).cloned()
}

#[instrument(level = "debug")]
pub async fn remove_camera(uuid: &Uuid) -> Result<Camera> {
    let mut lock = MANAGER.get().unwrap().write().await;

    lock.cameras.swap_remove(uuid).context("context")
}

#[instrument(level = "debug")]
pub async fn clear_cameras() {
    let mut lock = MANAGER.get().unwrap().write().await;

    lock.cameras.clear();
}

#[tokio::test]
async fn test_camera_manager_full_cycle() {
    let mcm_address = "127.0.0.1:6021".parse().unwrap();
    init(mcm_address).await;

    let test_camera = Camera {
        uuid: "bc071801-c50f-8301-ac36-bc071801c50f".parse().unwrap(),
        hostname: "192.168.0.200".parse().unwrap(),
        credentials: Some(Credentials {
            username: "test_user".to_string(),
            password: "test_password".to_string(),
        }),
        streams: IndexMap::new(),
    };

    // Add the test camera
    add_camera(&test_camera).await.unwrap();

    // Verify the camera was added
    let all_cameras = cameras().await;
    assert_eq!(all_cameras.len(), 1);
    assert!(all_cameras.contains_key(&test_camera.uuid));

    // Retrieve the test camera
    let retrieved_camera = get_camera(&test_camera.uuid).await;
    assert!(retrieved_camera.is_some());
    assert_eq!(retrieved_camera.unwrap(), test_camera);

    // Remove the test camera
    let removed_camera = remove_camera(&test_camera.uuid).await.unwrap();
    assert_eq!(removed_camera, test_camera);

    // Verify the camera was removed
    let all_cameras = cameras().await;
    assert_eq!(all_cameras.len(), 0);
    assert!(!all_cameras.contains_key(&test_camera.uuid));
}
