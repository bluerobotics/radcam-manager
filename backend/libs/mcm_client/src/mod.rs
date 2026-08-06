use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr},
    sync::Mutex,
    time::{Duration, Instant}, // std::time::Instant: sync health_state under Mutex
};

use anyhow::{Context, Result};
use indexmap::IndexMap;
use mcm_client::MCMClient;
use once_cell::sync::OnceCell;
use radcam_api::McmHealth;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{RwLock, broadcast},
    task::JoinHandle,
};
use tracing::*;
use ts_rs::TS;
use url::Url;
use uuid::Uuid;

// note: keep this private to isolate MCM API from the rest of the code
pub(crate) mod mcm_client;
pub mod mcm_types;

const MCM_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MCM_FAILURES_TO_DOWN: u32 = 3;

static MANAGER: OnceCell<RwLock<Manager>> = OnceCell::new();
static CAMERAS_TX: OnceCell<broadcast::Sender<()>> = OnceCell::new();
static HEALTH: OnceCell<Mutex<HealthState>> = OnceCell::new();
static HEALTH_TX: OnceCell<broadcast::Sender<()>> = OnceCell::new();
/// Survives a camera dropping out of discovery, so its HTTP API stays addressable.
static LAST_HOSTNAMES: OnceCell<Mutex<HashMap<Uuid, Ipv4Addr>>> = OnceCell::new();

struct HealthState {
    state: McmHealth,
    detail: Option<String>,
    consecutive_failures: u32,
    first_failure_at: Option<Instant>,
}

/// Point-in-time MCM link health for system-health aggregation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McmHealthSnapshot {
    /// Current [`McmHealth`] state (Unknown until the first poll completes).
    pub state: McmHealth,
    /// Human-readable failure or probe detail when unhealthy; `None` when online.
    pub detail: Option<String>,
    /// Consecutive failed MCM poll cycles since the last success.
    pub consecutive_failures: u32,
}

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
    /// Device login credentials; never serialized to API clients.
    #[serde(skip)]
    #[ts(skip)]
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
pub async fn init(mcm_address: SocketAddr, skip_hardware_check: bool) {
    if let Some(manager) = MANAGER.get() {
        let mut lock = manager.write().await;
        lock._authentication_task_handler.abort();
        lock._start_radcams_task_handler.abort();
        lock._authentication_task_handler =
            tokio::spawn(
                async move { authenticate_radcams(&mcm_address, skip_hardware_check).await },
            );
        lock._start_radcams_task_handler =
            tokio::spawn(async move { start_radcams_streams(&mcm_address).await });
        return;
    }

    let cameras = IndexMap::new();
    let _authentication_task_handler =
        tokio::spawn(async move { authenticate_radcams(&mcm_address, skip_hardware_check).await });
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

#[instrument(level = "debug")]
pub async fn shutdown() {
    if let Some(manager) = MANAGER.get() {
        let mut lock = manager.write().await;
        lock._authentication_task_handler.abort();
        lock._start_radcams_task_handler.abort();
        lock.cameras.clear();
    }
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
async fn authenticate_radcams(mcm_address: &SocketAddr, skip_hardware_check: bool) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let mcm = match tokio::time::timeout(
            MCM_PROBE_TIMEOUT,
            MCMClient::try_new(mcm_address, skip_hardware_check),
        )
        .await
        {
            Ok(Ok(mcm)) => mcm,
            Ok(Err(error)) => {
                debug!("Failed to create MCM client: {error:?}");
                report_failure(mcm_failure_detail(&error, mcm_address), *mcm_address);
                continue;
            }
            Err(_) => {
                report_failure(
                    "MCM did not answer /info within 5s".to_string(),
                    *mcm_address,
                );
                continue;
            }
        };

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let radcams = match tokio::time::timeout(MCM_PROBE_TIMEOUT, mcm.get_radcams()).await {
                Ok(Ok(radcams)) => {
                    report_success(mcm.address);
                    radcams
                }
                Ok(Err(error)) => {
                    debug!("Failed to get radcams: {error:?}");
                    report_failure(mcm_failure_detail(&error, &mcm.address), mcm.address);
                    break;
                }
                Err(_) => {
                    report_failure(
                        format!(
                            "MCM did not answer device list within 5s at {}",
                            mcm.address
                        ),
                        mcm.address,
                    );
                    break;
                }
            };

            let known_cameras = cameras().await;
            let radcam_uuids: std::collections::HashSet<Uuid> =
                radcams.iter().map(|camera| camera.uuid).collect();
            for uuid in known_cameras.keys() {
                if !radcam_uuids.contains(uuid)
                    && let Err(error) = remove_camera(uuid).await
                {
                    debug!("Failed removing stale camera {uuid}: {error:?}");
                }
            }

            for camera in &radcams {
                if let Some(known_camera) = known_cameras.get(&camera.uuid) {
                    if known_camera != camera
                        && let Err(error) = add_camera(camera).await
                    {
                        debug!("Failed updating camera {camera:?}: {error:?}");
                        continue;
                    }

                    continue;
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

        let mcm =
            match tokio::time::timeout(MCM_PROBE_TIMEOUT, MCMClient::try_new(mcm_address, false))
                .await
            {
                Ok(Ok(mcm)) => mcm,
                Ok(Err(error)) => {
                    debug!("Failed to create MCM client: {error:?}");
                    continue;
                }
                Err(_) => {
                    debug!("MCM try_new timed out in start_radcams_streams");
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
    let Some(manager) = MANAGER.get() else {
        return IndexMap::new();
    };
    manager.read().await.cameras.clone()
}

/// Subscribe to camera-list change notifications for WebSocket push events.
pub fn subscribe_cameras() -> broadcast::Receiver<()> {
    cameras_sender().subscribe()
}

/// Current MCM link health, detail text, and consecutive failing poll cycles.
pub fn health() -> McmHealthSnapshot {
    let guard = health_state().lock().expect("health lock");
    McmHealthSnapshot {
        state: guard.state,
        detail: guard.detail.clone(),
        consecutive_failures: guard.consecutive_failures,
    }
}

/// Subscribe to MCM health change notifications.
pub fn subscribe_health() -> broadcast::Receiver<()> {
    health_sender().subscribe()
}

fn health_state() -> &'static Mutex<HealthState> {
    HEALTH.get_or_init(|| {
        Mutex::new(HealthState {
            state: McmHealth::Unknown,
            detail: None,
            consecutive_failures: 0,
            first_failure_at: None,
        })
    })
}

fn health_sender() -> &'static broadcast::Sender<()> {
    HEALTH_TX.get_or_init(|| {
        let (sender, _) = broadcast::channel(16);
        sender
    })
}

fn notify_health() {
    if let Err(error) = health_sender().send(()) {
        debug!("No MCM health subscribers: {error}");
    }
}

fn is_connection_refused(error: &anyhow::Error) -> bool {
    for cause in error.chain() {
        if let Some(io_err) = cause.downcast_ref::<std::io::Error>()
            && io_err.kind() == std::io::ErrorKind::ConnectionRefused
        {
            return true;
        }
        if cause.to_string().contains("Connection refused") {
            // ponytail: string fallback when the error chain lacks a typed io::Error
            return true;
        }
    }
    false
}

fn mcm_failure_detail(error: &anyhow::Error, address: &SocketAddr) -> String {
    if is_connection_refused(error) {
        return format!("connection refused at {address}");
    }
    format!("{error}")
}

fn report_failure(detail: String, address: SocketAddr) {
    let mut guard = health_state().lock().expect("health lock");

    if guard.consecutive_failures == 0 {
        guard.first_failure_at = Some(Instant::now());
    }
    guard.consecutive_failures += 1;
    guard.detail = Some(detail.clone());

    let should_degrade = guard.consecutive_failures >= MCM_FAILURES_TO_DOWN
        && matches!(guard.state, McmHealth::Online | McmHealth::Unknown);

    if should_degrade {
        let elapsed = guard
            .first_failure_at
            .map(|at| at.elapsed())
            .unwrap_or_default();
        guard.state = McmHealth::Down;
        warn!(
            mcm_address = %address,
            detail = %detail,
            consecutive_failures = guard.consecutive_failures,
            elapsed_secs = elapsed.as_secs_f32(),
            "MCM health degraded to Down"
        );
        notify_health();
    } else if guard.state == McmHealth::Down {
        // Live attempt feedback for the sticky system-health dialog.
        notify_health();
    }
}

fn report_success(address: SocketAddr) {
    let mut guard = health_state().lock().expect("health lock");
    let was_online = guard.state == McmHealth::Online;
    guard.consecutive_failures = 0;
    guard.first_failure_at = None;
    guard.detail = None;

    if !was_online {
        guard.state = McmHealth::Online;
        info!(mcm_address = %address, "MCM health recovered to Online");
        notify_health();
    }
}

fn last_hostnames() -> &'static Mutex<HashMap<Uuid, Ipv4Addr>> {
    LAST_HOSTNAMES.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn remember_hostname(uuid: Uuid, hostname: Ipv4Addr) {
    last_hostnames()
        .lock()
        .expect("last hostnames lock")
        .insert(uuid, hostname);
}

pub fn forget_hostname(uuid: Uuid) {
    last_hostnames()
        .lock()
        .expect("last hostnames lock")
        .remove(&uuid);
}

pub fn cached_hostname(uuid: &Uuid) -> Option<Ipv4Addr> {
    last_hostnames()
        .lock()
        .expect("last hostnames lock")
        .get(uuid)
        .copied()
}

fn cameras_sender() -> &'static broadcast::Sender<()> {
    CAMERAS_TX.get_or_init(|| {
        let (sender, _) = broadcast::channel(16);
        sender
    })
}

fn notify_cameras() {
    if let Err(error) = cameras_sender().send(()) {
        debug!("No camera-list subscribers: {error}");
    }
}

#[instrument(level = "debug")]
pub async fn add_camera(camera: &Camera) -> Result<()> {
    let mut lock = MANAGER.get().unwrap().write().await;

    last_hostnames()
        .lock()
        .expect("last hostnames lock")
        .insert(camera.uuid, camera.hostname);

    if let Some(old_camera) = lock.cameras.insert(camera.uuid, camera.clone()) {
        debug!("Camera updated: old: {old_camera:?}");
    }

    notify_cameras();

    Ok(())
}

#[instrument(level = "debug")]
pub async fn get_camera(uuid: &Uuid) -> Option<Camera> {
    let manager = MANAGER.get()?.read().await;
    manager.cameras.get(uuid).cloned()
}

/// Address to reach `uuid` over HTTP, falling back to the last hostname it was seen at.
///
/// ONVIF rediscovery drops cameras from the MCM list for a while after the video service
/// restarts; the camera itself keeps answering, so control must not wait for discovery.
#[instrument(level = "debug")]
pub async fn camera_address(uuid: &Uuid) -> Option<Ipv4Addr> {
    if let Some(camera) = get_camera(uuid).await {
        return Some(camera.hostname);
    }
    last_hostnames()
        .lock()
        .expect("last hostnames lock")
        .get(uuid)
        .copied()
}

#[instrument(level = "debug")]
pub async fn remove_camera(uuid: &Uuid) -> Result<Camera> {
    let mut lock = MANAGER.get().unwrap().write().await;

    let camera = lock.cameras.swap_remove(uuid).context("context")?;
    notify_cameras();

    Ok(camera)
}

#[cfg(test)]
// Resets in-process MCM health counters for unit tests. The process-lifetime
// OnceCell is not cleared — tests share globals and must reset observable
// state or use disjoint inputs (see S-T6).
fn health_reset() {
    let mut guard = health_state().lock().expect("health lock");
    guard.state = McmHealth::Unknown;
    guard.detail = None;
    guard.consecutive_failures = 0;
    guard.first_failure_at = None;
}

#[test]
fn health_needs_three_failures() {
    health_reset();
    let address: SocketAddr = "127.0.0.1:6021".parse().unwrap();

    report_success(address);
    assert_eq!(health().state, McmHealth::Online);

    let refused = format!("connection refused at {address}");
    report_failure(refused.clone(), address);
    assert_eq!(health().state, McmHealth::Online);
    assert_eq!(health().consecutive_failures, 1);

    report_failure(refused.clone(), address);
    assert_eq!(health().state, McmHealth::Online);
    assert_eq!(health().consecutive_failures, 2);

    report_failure(refused.clone(), address);
    assert_eq!(health().state, McmHealth::Down);
    assert_eq!(health().detail, Some(refused));

    report_success(address);
    assert_eq!(health().state, McmHealth::Online);
    assert_eq!(health().consecutive_failures, 0);
    assert_eq!(health().detail, None);

    health_reset();
    report_success(address);
    for _ in 0..3 {
        report_failure("MCM did not answer /info within 5s".to_string(), address);
    }
    assert_eq!(health().state, McmHealth::Down);
    assert_eq!(
        health().detail,
        Some("MCM did not answer /info within 5s".to_string())
    );
}

#[tokio::test]
async fn test_camera_manager_full_cycle() {
    let mcm_address = "127.0.0.1:6021".parse().unwrap();
    init(mcm_address, false).await;
    init(mcm_address, false).await;
    shutdown().await;
    init(mcm_address, false).await;

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

    // Dropping out of discovery must not make the camera unaddressable: its HTTP API
    // keeps answering while ONVIF rediscovers it.
    assert_eq!(
        camera_address(&test_camera.uuid).await,
        Some(test_camera.hostname)
    );
    assert_eq!(camera_address(&Uuid::nil()).await, None);
}
