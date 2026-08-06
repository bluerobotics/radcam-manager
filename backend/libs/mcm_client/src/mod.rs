use std::{
    collections::{HashMap, HashSet},
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

use mcm_types::StreamStatusState;

// note: keep this private to isolate MCM API from the rest of the code
pub(crate) mod mcm_client;
pub mod mcm_types;

const MCM_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MCM_FAILURES_TO_DOWN: u32 = 3;
// MCM self-heals with backoff capped at 60s; each recreate costs an MCM settings write.
const STREAM_FAILURES_TO_RECREATE: u32 = 60;

static MANAGER: OnceCell<RwLock<Manager>> = OnceCell::new();
static CAMERAS_TX: OnceCell<broadcast::Sender<()>> = OnceCell::new();
static HEALTH: OnceCell<Mutex<HealthState>> = OnceCell::new();
static HEALTH_TX: OnceCell<broadcast::Sender<()>> = OnceCell::new();
/// Per-camera hostname retained after the camera drops out of MCM discovery so its HTTP
/// API stays addressable; entries are only removed on an explicit Forget, not on
/// `remove_camera`.
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
struct StreamFailure {
    episode_key: String,
    detail: String,
}

#[derive(Debug)]
struct Manager {
    cameras: Cameras,
    auth_failures: HashMap<Uuid, String>,
    stream_failures: HashMap<Uuid, StreamFailure>,
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
    state: StreamStatusState,
    error: Option<String>,
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
            auth_failures: HashMap::new(),
            stream_failures: HashMap::new(),
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
        lock.auth_failures.clear();
        lock.stream_failures.clear();
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
            let radcam_uuids: HashSet<Uuid> = radcams.iter().map(|camera| camera.uuid).collect();
            prune_auth_failures(&radcam_uuids).await;
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
                    record_auth_failure(camera.uuid, error.to_string()).await;
                    debug!("Failed authenticating onvif camera {camera:?}: {error:?}");
                    continue;
                }

                clear_auth_failure(&camera.uuid).await;

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
    let mut stream_failure_counts: HashMap<Url, u32> = HashMap::new();

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

            let mut observed_sources = HashSet::new();

            for source in available_radcam_sources {
                if !source.source.ends_with("stream_0") {
                    continue; // We only want the main stream
                }

                let Ok(mut available_source) = source.source.parse::<Url>() else {
                    warn!(
                        "Skipping video source with unparsable URL: {:?}",
                        source.source
                    );
                    continue;
                };

                // Note: Here we are ignoring any authentication so we avoid duplicated streams.
                // Stripping fails only on URLs that cannot carry credentials, which are already
                // credential-free.
                let _ = available_source.set_password(None);
                let _ = available_source.set_username("");
                observed_sources.insert(available_source.clone());

                let matching_stream = existing_radcam_streams.iter().find(|stream| {
                    let mut existing_source = stream.source_endpoint.clone();
                    let _ = existing_source.set_password(None);
                    let _ = existing_source.set_username("");

                    existing_source.eq(&available_source)
                });

                if let Some(stream) = matching_stream {
                    if stream_needs_recreation(stream.state) {
                        let count = stream_failure_counts
                            .entry(available_source.clone())
                            .or_insert(0);
                        *count += 1;

                        if *count >= STREAM_FAILURES_TO_RECREATE {
                            set_stream_failure_for_source(
                                &available_source,
                                stream.state,
                                stream.error.as_deref(),
                            )
                            .await;
                            match mcm.delete_stream(&stream.name).await {
                                Err(error) => {
                                    warn!(
                                        "Failed deleting failed stream {:?}: {error:?}",
                                        stream.name
                                    );
                                }
                                Ok(_) => {
                                    *count = 0;
                                    if let Err(error) = mcm.create_stream(source).await {
                                        warn!("Failed recreating stream: {error:?}");
                                    }
                                }
                            }
                        }
                    } else {
                        stream_failure_counts.remove(&available_source);
                        clear_stream_failure_for_source(&available_source).await;
                    }

                    continue;
                }

                stream_failure_counts.remove(&available_source);

                if let Err(error) = mcm.create_stream(source).await {
                    warn!("Failed creating stream: {error:?}");
                    continue;
                }
            }

            stream_failure_counts.retain(|source, _| observed_sources.contains(source));
        }
    }
}

/// Per-camera ONVIF authentication failure, when MCM discovery sees the camera but login fails.
pub async fn authentication_failure(uuid: &Uuid) -> Option<String> {
    let manager = MANAGER.get()?;
    manager.read().await.auth_failures.get(uuid).cloned()
}

/// Per-camera video stream failure, when MCM's stream stayed broken through self-heal backoff.
pub async fn stream_failure(uuid: &Uuid) -> Option<String> {
    let manager = MANAGER.get()?;
    manager
        .read()
        .await
        .stream_failures
        .get(uuid)
        .map(|failure| failure.detail.clone())
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
    lock.stream_failures.remove(uuid);
    notify_cameras();

    Ok(camera)
}

fn stream_needs_recreation(state: StreamStatusState) -> bool {
    state == StreamStatusState::Stopped
}

async fn record_auth_failure(uuid: Uuid, error: String) {
    let Some(manager) = MANAGER.get() else {
        return;
    };
    let changed = {
        let mut lock = manager.write().await;
        if lock.auth_failures.get(&uuid) == Some(&error) {
            false
        } else {
            lock.auth_failures.insert(uuid, error);
            true
        }
    };
    if changed {
        notify_cameras();
    }
}

async fn clear_auth_failure(uuid: &Uuid) {
    let Some(manager) = MANAGER.get() else {
        return;
    };
    let changed = manager.write().await.auth_failures.remove(uuid).is_some();
    if changed {
        notify_cameras();
    }
}

async fn prune_auth_failures(visible: &HashSet<Uuid>) {
    let Some(manager) = MANAGER.get() else {
        return;
    };
    let changed = {
        let mut lock = manager.write().await;
        let before = lock.auth_failures.len();
        lock.auth_failures.retain(|uuid, _| visible.contains(uuid));
        lock.auth_failures.len() != before
    };
    if changed {
        notify_cameras();
    }
}

async fn set_stream_failure_for_source(
    source: &Url,
    state: StreamStatusState,
    error: Option<&str>,
) {
    let Some(uuid) = camera_uuid_for_stream_source(source).await else {
        return;
    };
    let episode_key = stream_failure_episode_key(state);
    let detail = stream_failure_detail(error);
    let Some(manager) = MANAGER.get() else {
        return;
    };
    let changed = {
        let mut lock = manager.write().await;
        match lock.stream_failures.get(&uuid) {
            Some(existing) if existing.episode_key == episode_key => {
                if existing.detail != detail
                    && let Some(record) = lock.stream_failures.get_mut(&uuid)
                {
                    record.detail = detail;
                }
                false
            }
            _ => {
                lock.stream_failures.insert(
                    uuid,
                    StreamFailure {
                        episode_key,
                        detail,
                    },
                );
                true
            }
        }
    };
    if changed {
        notify_cameras();
    }
}

async fn clear_stream_failure_for_source(source: &Url) {
    let Some(uuid) = camera_uuid_for_stream_source(source).await else {
        return;
    };
    let Some(manager) = MANAGER.get() else {
        return;
    };
    if !manager.read().await.stream_failures.contains_key(&uuid) {
        return;
    }
    let changed = manager
        .write()
        .await
        .stream_failures
        .remove(&uuid)
        .is_some();
    if changed {
        notify_cameras();
    }
}

fn stream_failure_episode_key(state: StreamStatusState) -> String {
    format!("{state:?}")
}

fn stream_failure_detail(error: Option<&str>) -> String {
    error
        .map(str::to_string)
        .unwrap_or_else(|| "MCM stream state: stopped".to_string())
}

async fn camera_uuid_for_stream_source(source: &Url) -> Option<Uuid> {
    let host = source.host_str()?;
    let ip = host.parse::<Ipv4Addr>().ok()?;
    let manager = MANAGER.get()?;
    manager
        .read()
        .await
        .cameras
        .iter()
        .find(|(_, camera)| camera.hostname == ip)
        .map(|(uuid, _)| *uuid)
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

#[test]
fn stream_needs_recreation_predicate() {
    // Only Stopped means current failure; Idle may still carry a stale MCM error string.
    assert!(!stream_needs_recreation(StreamStatusState::Idle));
    assert!(!stream_needs_recreation(StreamStatusState::Running));
    assert!(!stream_needs_recreation(StreamStatusState::Unknown));
    assert!(stream_needs_recreation(StreamStatusState::Stopped));
}
