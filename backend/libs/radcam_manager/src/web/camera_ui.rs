//! Lock order: [`UI`] is always taken before `camera_state::REGISTRY`. Every mutation
//! therefore releases the [`UI`] guard before calling [`camera_state::emit_ui`].

use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
    time::{Duration, Instant},
};

use autopilot::api::{Action as AutopilotAction, ActuatorsConfig};
use once_cell::sync::OnceCell;
use radcam_api::{CameraConnectivity, CameraUiState, OnePushAwbStatus, UiDismissField};
use radcam_commands::Action as CameraAction;
use tokio::task::JoinHandle;
use tracing::*;
use uuid::Uuid;

use crate::web::camera_state;

const REBOOT_GRACE: Duration = Duration::from_secs(180);
const LOADING_GRACE: Duration = Duration::from_secs(120);
const MIN_LOADING: Duration = Duration::from_secs(3);

static UI: OnceCell<Mutex<HashMap<Uuid, Entry>>> = OnceCell::new();

struct Entry {
    state: CameraUiState,
    loading_count: usize,
    loading_started_at: Option<Instant>,
    reboot_timeout: Option<JoinHandle<()>>,
    loading_timeout: Option<JoinHandle<()>>,
    min_loading_timeout: Option<JoinHandle<()>>,
}

/// Current UI state for `camera_uuid`, or defaults when unset.
#[instrument(level = "debug")]
pub(crate) fn get(camera_uuid: Uuid) -> CameraUiState {
    ui().lock()
        .unwrap()
        .get(&camera_uuid)
        .map(|entry| entry.state.clone())
        .unwrap_or_default()
}

/// Drop UI entries for cameras that are no longer present.
///
/// Entries grow with every camera ever touched; prune when the MCM list shrinks.
#[instrument(level = "debug", skip_all)]
pub(crate) fn retain_known_cameras(known: &HashSet<Uuid>) {
    let mut lock = ui().lock().unwrap();
    lock.retain(|camera_uuid, entry| {
        if known.contains(camera_uuid) {
            return true;
        }
        clear_reboot_timeout(entry);
        clear_loading_timeout(entry);
        clear_min_loading_timeout(entry);
        false
    });
}

/// Set or clear the shared one-push AWB phase for all clients.
#[instrument(level = "debug")]
pub(crate) fn set_one_push_awb(camera_uuid: Uuid, status: Option<OnePushAwbStatus>) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let entry = lock.entry(camera_uuid).or_insert_with(new_entry);
        entry.state.one_push_awb = status;
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

/// Set or clear the shared connectivity state for all clients.
#[instrument(level = "debug")]
pub(crate) fn set_connectivity(camera_uuid: Uuid, connectivity: CameraConnectivity) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let entry = lock.entry(camera_uuid).or_insert_with(new_entry);
        if entry.state.connectivity == connectivity {
            return;
        }
        entry.state.connectivity = connectivity;
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

/// Set or clear the shared video stream error for all clients.
#[instrument(level = "debug")]
pub(crate) fn set_stream_error(camera_uuid: Uuid, error: Option<String>) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let entry = lock.entry(camera_uuid).or_insert_with(new_entry);
        if entry.state.stream_error == error {
            return;
        }
        entry.state.stream_error = error;
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

/// Set or clear the shared ONVIF authentication error for all clients.
#[instrument(level = "debug")]
pub(crate) fn set_onvif_auth_error(camera_uuid: Uuid, error: Option<String>) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let entry = lock.entry(camera_uuid).or_insert_with(new_entry);
        if entry.state.onvif_auth_error == error {
            return;
        }
        entry.state.onvif_auth_error = error;
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

/// Dismiss a UI overlay field for all clients.
#[instrument(level = "debug")]
pub(crate) fn dismiss(camera_uuid: Uuid, field: UiDismissField) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let Some(entry) = lock.get_mut(&camera_uuid) else {
            return;
        };
        match field {
            UiDismissField::ErrorDialog => entry.state.error_dialog = None,
            UiDismissField::WarningToast => entry.state.warning_toast = None,
        }
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

/// Handle a failed camera control: dialog for deliberate actions, toast otherwise.
#[instrument(level = "debug", skip(action))]
pub(crate) fn fail_camera_action(camera_uuid: Uuid, action: &CameraAction, error: &str) {
    if is_reboot_camera_action(action) {
        end_reboot(camera_uuid);
        set_error(
            camera_uuid,
            format!("{}: {error}", camera_action_label(action)),
        );
        return;
    }

    if loading_message_for_camera_action(action).is_some() {
        end_loading(camera_uuid);
        set_error(
            camera_uuid,
            format!("{}: {error}", camera_action_label(action)),
        );
        return;
    }

    set_warning(camera_uuid, format!("Camera control failed: {error}"));
}

/// Handle a failed autopilot control: dialog for deliberate actions, toast otherwise.
#[instrument(level = "debug", skip(action))]
pub(crate) fn fail_autopilot_action(camera_uuid: Uuid, action: &AutopilotAction, error: &str) {
    if loading_message_for_autopilot_action(action).is_some() {
        end_loading(camera_uuid);
        set_error(
            camera_uuid,
            format!("{}: {error}", autopilot_action_label(action)),
        );
        return;
    }

    set_warning(camera_uuid, format!("Autopilot control failed: {error}"));
}

/// Start UI lifecycle for a camera action, if it is a deliberate long-running one.
#[instrument(level = "debug", skip(action))]
pub(crate) fn start_camera_action(camera_uuid: Uuid, action: &CameraAction) {
    let Some(message) = loading_message_for_camera_action(action) else {
        return;
    };
    if is_reboot_camera_action(action) {
        begin_reboot(camera_uuid, message);
    } else {
        begin_loading(camera_uuid, message);
    }
}

/// Finish UI lifecycle for a successful camera action.
#[instrument(level = "debug", skip(action))]
pub(crate) fn finish_camera_action(camera_uuid: Uuid, action: &CameraAction) {
    if is_reboot_camera_action(action) {
        // handle_control already waited for offline → online; clear the overlay.
        end_reboot(camera_uuid);
        return;
    }
    if loading_message_for_camera_action(action).is_some() {
        end_loading(camera_uuid);
    }
}

/// Start UI lifecycle for an autopilot action, if deliberate.
#[instrument(level = "debug", skip(action))]
pub(crate) fn start_autopilot_action(camera_uuid: Uuid, action: &AutopilotAction) {
    let Some(message) = loading_message_for_autopilot_action(action) else {
        return;
    };
    begin_loading(camera_uuid, message);
}

/// Finish UI lifecycle for a successful autopilot action.
#[instrument(level = "debug", skip(action))]
pub(crate) fn finish_autopilot_action(camera_uuid: Uuid, action: &AutopilotAction) {
    if loading_message_for_autopilot_action(action).is_some() {
        end_loading(camera_uuid);
    }
}

/// Begin a nested loading overlay for a deliberate long-running action.
#[instrument(level = "debug")]
fn begin_loading(camera_uuid: Uuid, message: &str) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let entry = lock.entry(camera_uuid).or_insert_with(new_entry);
        clear_min_loading_timeout(entry);
        entry.loading_count += 1;
        if entry.loading_count == 1 {
            entry.loading_started_at = Some(Instant::now());
        }
        entry.state.loading = true;
        entry.state.loading_message = Some(message.to_string());
        entry.state.error_dialog = None;
        arm_loading_timeout(entry, camera_uuid);
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

/// End one nested loading overlay.
#[instrument(level = "debug")]
fn end_loading(camera_uuid: Uuid) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let Some(entry) = lock.get_mut(&camera_uuid) else {
            return;
        };
        entry.loading_count = entry.loading_count.saturating_sub(1);
        if entry.loading_count > 0 {
            return;
        }
        clear_loading_timeout(entry);
        clear_min_loading_timeout(entry);

        let remaining = entry
            .loading_started_at
            .map(|started| MIN_LOADING.saturating_sub(started.elapsed()))
            .unwrap_or(Duration::ZERO);

        if !remaining.is_zero() {
            // Keep the overlay up so the user can read the message.
            entry.min_loading_timeout = Some(tokio::spawn(
                async move {
                    tokio::time::sleep(remaining).await;
                    finish_min_loading(camera_uuid);
                }
                .instrument(Span::current()),
            ));
            return;
        }

        entry.loading_started_at = None;
        entry.state.loading = false;
        entry.state.loading_message = None;
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

#[instrument(level = "debug")]
fn finish_min_loading(camera_uuid: Uuid) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let Some(entry) = lock.get_mut(&camera_uuid) else {
            return;
        };
        if entry.loading_count > 0 || entry.state.rebooting || !entry.state.loading {
            return;
        }
        clear_min_loading_timeout(entry);
        clear_loading_timeout(entry);
        entry.loading_started_at = None;
        entry.state.loading = false;
        entry.state.loading_message = None;
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

#[instrument(level = "debug")]
fn force_end_loading(camera_uuid: Uuid) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let Some(entry) = lock.get_mut(&camera_uuid) else {
            return;
        };
        if entry.loading_count == 0 && !entry.state.loading {
            return;
        }
        clear_loading_timeout(entry);
        clear_min_loading_timeout(entry);
        entry.loading_count = 0;
        entry.loading_started_at = None;
        if !entry.state.rebooting {
            entry.state.loading = false;
            entry.state.loading_message = None;
        }
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

/// Begin reboot overlay with a grace timeout covering offline + online waits.
#[instrument(level = "debug")]
fn begin_reboot(camera_uuid: Uuid, message: &str) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let entry = lock.entry(camera_uuid).or_insert_with(new_entry);
        clear_reboot_timeout(entry);
        clear_loading_timeout(entry);
        clear_min_loading_timeout(entry);
        entry.loading_count = 0;
        entry.loading_started_at = None;
        entry.state.loading = true;
        entry.state.loading_message = Some(message.to_string());
        entry.state.rebooting = true;
        entry.state.warning_toast = None;
        entry.state.error_dialog = None;
        entry.reboot_timeout = Some(tokio::spawn(
            async move {
                tokio::time::sleep(REBOOT_GRACE).await;
                end_reboot(camera_uuid);
            }
            .instrument(Span::current()),
        ));
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

/// Clear reboot overlay once the camera is responsive again.
#[instrument(level = "debug")]
fn end_reboot(camera_uuid: Uuid) {
    let state = {
        let mut lock = ui().lock().unwrap();
        let Some(entry) = lock.get_mut(&camera_uuid) else {
            return;
        };
        if !entry.state.rebooting {
            return;
        }
        clear_reboot_timeout(entry);
        clear_loading_timeout(entry);
        clear_min_loading_timeout(entry);
        entry.loading_count = 0;
        entry.loading_started_at = None;
        entry.state.rebooting = false;
        entry.state.loading = false;
        entry.state.loading_message = None;
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

/// Show a modal error dialog without clearing an in-progress reboot.
#[instrument(level = "debug")]
fn set_error(camera_uuid: Uuid, message: String) {
    if autopilot::error_indicates_actuators_not_configured(&message) {
        return;
    }

    let state = {
        let mut lock = ui().lock().unwrap();
        let entry = lock.entry(camera_uuid).or_insert_with(new_entry);
        clear_loading_timeout(entry);
        clear_min_loading_timeout(entry);
        entry.loading_count = 0;
        entry.loading_started_at = None;
        if !entry.state.rebooting {
            entry.state.loading = false;
            entry.state.loading_message = None;
        }
        entry.state.error_dialog = Some(message);
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

/// Show a transient warning toast; suppressed while rebooting.
#[instrument(level = "debug")]
pub(crate) fn set_warning(camera_uuid: Uuid, message: String) {
    if autopilot::error_indicates_actuators_not_configured(&message) {
        return;
    }

    let state = {
        let mut lock = ui().lock().unwrap();
        let entry = lock.entry(camera_uuid).or_insert_with(new_entry);
        if entry.state.rebooting {
            return;
        }
        entry.state.warning_toast = Some(message);
        entry.state.clone()
    };
    camera_state::emit_ui(camera_uuid, state);
}

fn is_reboot_camera_action(action: &CameraAction) -> bool {
    matches!(action, CameraAction::Restart)
}

fn loading_message_for_camera_action(action: &CameraAction) -> Option<&'static str> {
    match action {
        CameraAction::Restart => Some("Rebooting camera…"),
        CameraAction::SetRecommendedCameraSettings => Some("Applying recommended camera settings…"),
        _ => None,
    }
}

fn loading_message_for_autopilot_action(action: &AutopilotAction) -> Option<&'static str> {
    match action {
        AutopilotAction::ExportLuaScript => Some("Updating Lua script…"),
        AutopilotAction::ResetActuatorsConfig => Some("Applying default hardware setup…"),
        AutopilotAction::SetActuatorsConfig(config) if is_full_hardware_setup(config) => {
            Some("Applying custom hardware setup…")
        }
        AutopilotAction::ForgetActuatorsConfig => Some("Forgetting camera…"),
        _ => None,
    }
}

fn is_full_hardware_setup(config: &ActuatorsConfig) -> bool {
    let Some(parameters) = &config.parameters else {
        return false;
    };
    parameters.focus_channel.is_some()
        && parameters.zoom_channel.is_some()
        && parameters.tilt_channel.is_some()
        && parameters.script_channel.is_some()
}

fn camera_action_label(action: &CameraAction) -> &'static str {
    match action {
        CameraAction::Restart => "Failed to reboot camera",
        CameraAction::SetRecommendedCameraSettings => "Failed to apply recommended camera settings",
        _ => "Camera control failed",
    }
}

fn autopilot_action_label(action: &AutopilotAction) -> &'static str {
    match action {
        AutopilotAction::ExportLuaScript => "Failed to update Lua script",
        AutopilotAction::ResetActuatorsConfig => "Failed to apply default hardware setup",
        AutopilotAction::SetActuatorsConfig(_) => "Error saving hardware setup",
        _ => "Autopilot control failed",
    }
}

fn ui() -> &'static Mutex<HashMap<Uuid, Entry>> {
    UI.get_or_init(|| Mutex::new(HashMap::new()))
}

fn new_entry() -> Entry {
    Entry {
        state: CameraUiState::default(),
        loading_count: 0,
        loading_started_at: None,
        reboot_timeout: None,
        loading_timeout: None,
        min_loading_timeout: None,
    }
}

fn clear_reboot_timeout(entry: &mut Entry) {
    if let Some(handle) = entry.reboot_timeout.take() {
        handle.abort();
    }
}

fn clear_loading_timeout(entry: &mut Entry) {
    if let Some(handle) = entry.loading_timeout.take() {
        handle.abort();
    }
}

fn clear_min_loading_timeout(entry: &mut Entry) {
    if let Some(handle) = entry.min_loading_timeout.take() {
        handle.abort();
    }
}

fn arm_loading_timeout(entry: &mut Entry, camera_uuid: Uuid) {
    clear_loading_timeout(entry);
    entry.loading_timeout = Some(tokio::spawn(
        async move {
            tokio::time::sleep(LOADING_GRACE).await;
            force_end_loading(camera_uuid);
        }
        .instrument(Span::current()),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use autopilot::api::Action as AutopilotAction;

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn loading_overlay_lasts_at_least_three_seconds() {
        let camera_uuid = Uuid::from_u128(0x1111_2222_3333_4444);
        start_autopilot_action(camera_uuid, &AutopilotAction::ExportLuaScript);
        assert!(get(camera_uuid).loading);

        finish_autopilot_action(camera_uuid, &AutopilotAction::ExportLuaScript);
        assert!(get(camera_uuid).loading);

        tokio::time::advance(Duration::from_millis(500)).await;
        assert!(get(camera_uuid).loading);

        tokio::time::advance(MIN_LOADING).await;
        tokio::task::yield_now().await;
        assert!(!get(camera_uuid).loading);

        // Drop only this test's entry so parallel tests sharing the UI map stay isolated.
        let keep = {
            ui().lock()
                .unwrap()
                .keys()
                .copied()
                .filter(|id| *id != camera_uuid)
                .collect()
        };
        retain_known_cameras(&keep);
    }
}
