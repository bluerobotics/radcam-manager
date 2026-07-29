//! Backend-owned one-push white balance lifecycle.
//!
//! Pure settle logic is unit-tested; the tracker polls base parameters and drives
//! [`camera_ui`] / [`camera_state`] so every client stays in sync.

use std::{collections::HashMap, sync::Mutex, time::Instant};

use once_cell::sync::OnceCell;
use radcam_api::{CameraStateEvent, OnePushAwbPhase, OnePushAwbStatus};
use radcam_commands::{
    Action as CameraAction, CameraControl,
    protocol::display::base_display::{BaseAutoWhiteBalanceModeValue, BaseParameterSetting},
};
use tokio::task::JoinHandle;
use tracing::*;
use uuid::Uuid;

use crate::web::{camera_state, camera_ui};

/// Consecutive identical RGB samples required after movement before settle (Auto path).
pub const STABLE_SAMPLES: u8 = 2;
/// Wall-clock ceiling for a Running phase.
pub const RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
/// How long clients keep WB controls disabled after settle.
pub const COOLDOWN: std::time::Duration = std::time::Duration::from_secs(3);
/// Poll interval while Running.
pub const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbSample {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleMachine {
    start_rgb: Option<RgbSample>,
    last_rgb: Option<RgbSample>,
    saw_change: bool,
    stable_count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserveResult {
    Continue,
    Settled,
}

struct TrackerEntry {
    handle: JoinHandle<()>,
}

static TRACKERS: OnceCell<Mutex<HashMap<Uuid, TrackerEntry>>> = OnceCell::new();

impl RgbSample {
    pub fn from_base(base: &BaseParameterSetting) -> Option<Self> {
        Some(Self {
            red: base.awb_red?,
            green: base.awb_green?,
            blue: base.awb_blue?,
        })
    }
}

impl SettleMachine {
    pub fn begin(start: &BaseParameterSetting) -> Self {
        let rgb = RgbSample::from_base(start);
        Self {
            start_rgb: rgb,
            last_rgb: rgb,
            saw_change: false,
            stable_count: 0,
        }
    }

    /// Feed a base-parameter sample. Settles on Manual-after-change or RGB stability.
    pub fn observe(&mut self, sample: &BaseParameterSetting) -> ObserveResult {
        let rgb = RgbSample::from_base(sample);
        if let Some(current) = rgb {
            if let Some(last) = self.last_rgb {
                if current != last {
                    self.saw_change = true;
                    self.stable_count = 0;
                } else if self.saw_change {
                    self.stable_count = self.stable_count.saturating_add(1);
                }
            }
            if let Some(start) = self.start_rgb
                && current != start
            {
                self.saw_change = true;
            }
            self.last_rgb = Some(current);
        }

        let manual = matches!(sample.auto_awb, Some(BaseAutoWhiteBalanceModeValue::Manual));

        if self.saw_change && manual {
            return ObserveResult::Settled;
        }
        if self.saw_change && self.stable_count >= STABLE_SAMPLES {
            return ObserveResult::Settled;
        }
        ObserveResult::Continue
    }
}

/// True while Running or Cooldown for this camera.
pub(crate) fn is_busy(camera_uuid: Uuid) -> bool {
    camera_ui::get(camera_uuid).one_push_awb.is_some()
}

/// True when `action` is a one-push AWB trigger.
pub(crate) fn action_triggers_once_awb(action: &CameraAction) -> bool {
    match action {
        CameraAction::SetImageAdjustmentEx(params)
        | CameraAction::SetImageAdjustmentExAll(params) => params.once_awb == Some(1),
        _ => false,
    }
}

/// Abort the poll task and clear UI phase (camera gone / pruned).
#[instrument(level = "debug")]
pub(crate) fn abort(camera_uuid: Uuid) {
    if let Some(entry) = trackers().lock().unwrap().remove(&camera_uuid) {
        entry.handle.abort();
    }
    camera_ui::set_one_push_awb(camera_uuid, None);
}

/// Abort every tracker whose camera is no longer known.
#[instrument(level = "debug", skip_all)]
pub(crate) fn retain_known_cameras(known: &std::collections::HashSet<Uuid>) {
    let stale: Vec<Uuid> = {
        let lock = trackers().lock().unwrap();
        lock.keys()
            .copied()
            .filter(|uuid| !known.contains(uuid))
            .collect()
    };
    for uuid in stale {
        abort(uuid);
    }
}

/// Start Running after a successful onceAWB SET, or no-op if already busy.
#[instrument(level = "debug")]
pub(crate) fn begin_after_trigger(camera_uuid: Uuid) {
    if is_busy(camera_uuid) {
        return;
    }

    {
        let mut lock = trackers().lock().unwrap();
        if let Some(entry) = lock.remove(&camera_uuid) {
            entry.handle.abort();
        }
    }

    camera_ui::set_one_push_awb(
        camera_uuid,
        Some(OnePushAwbStatus {
            phase: OnePushAwbPhase::Running,
        }),
    );

    let handle = tokio::spawn(run_tracker(camera_uuid).instrument(Span::current()));
    trackers()
        .lock()
        .unwrap()
        .insert(camera_uuid, TrackerEntry { handle });
}

/// After a successful control that included onceAWB, start trackers.
#[instrument(level = "debug", skip_all, fields(%camera_uuid))]
pub(crate) fn on_control_success(camera_uuid: Uuid, action: &CameraAction) {
    if !action_triggers_once_awb(action) {
        return;
    }

    match action {
        CameraAction::SetImageAdjustmentEx(_) => {
            begin_after_trigger(camera_uuid);
        }
        CameraAction::SetImageAdjustmentExAll(_) => {
            let span = Span::current();
            tokio::spawn(
                async move {
                    let cameras = mcm_client::cameras().await;
                    for uuid in cameras.keys().copied() {
                        begin_after_trigger(uuid);
                    }
                }
                .instrument(span),
            );
        }
        _ => {}
    }
}

fn trackers() -> &'static Mutex<HashMap<Uuid, TrackerEntry>> {
    TRACKERS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[instrument(level = "debug", skip_all, fields(%camera_uuid))]
async fn run_tracker(camera_uuid: Uuid) {
    let start = Instant::now();
    let initial = match fetch_base_setting(camera_uuid).await {
        Ok(base) => base,
        Err(error) => {
            warn!("one-push AWB start snapshot failed: {error}");
            finish_timeout(camera_uuid);
            return;
        }
    };

    // Push the start snapshot so clients see gains as soon as the run begins.
    if let Ok(value) = serde_json::to_value(&initial) {
        camera_state::emit(CameraStateEvent {
            camera_uuid,
            base_parameters: Some(value),
            ..Default::default()
        });
    }

    let mut machine = SettleMachine::begin(&initial);
    let mut last_emitted = initial.clone();

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;

        if start.elapsed() >= RUN_TIMEOUT {
            finish_timeout(camera_uuid);
            return;
        }

        let sample = match fetch_base_setting(camera_uuid).await {
            Ok(base) => base,
            Err(error) => {
                warn!("one-push AWB poll failed: {error}");
                continue;
            }
        };

        if sample != last_emitted
            && let Ok(value) = serde_json::to_value(&sample)
        {
            camera_state::emit(CameraStateEvent {
                camera_uuid,
                base_parameters: Some(value),
                ..Default::default()
            });
            last_emitted = sample.clone();
        }

        if machine.observe(&sample) == ObserveResult::Settled {
            finish_settled(camera_uuid).await;
            return;
        }
    }
}

async fn finish_settled(camera_uuid: Uuid) {
    camera_ui::set_one_push_awb(
        camera_uuid,
        Some(OnePushAwbStatus {
            phase: OnePushAwbPhase::Cooldown,
        }),
    );
    tokio::time::sleep(COOLDOWN).await;
    // Only clear if we are still the cooldown owner (no newer run).
    if matches!(
        camera_ui::get(camera_uuid)
            .one_push_awb
            .as_ref()
            .map(|status| status.phase),
        Some(OnePushAwbPhase::Cooldown)
    ) {
        camera_ui::set_one_push_awb(camera_uuid, None);
    }
    trackers().lock().unwrap().remove(&camera_uuid);
}

fn finish_timeout(camera_uuid: Uuid) {
    camera_ui::set_warning(camera_uuid, "One-push white balance timed out".to_string());
    camera_ui::set_one_push_awb(camera_uuid, None);
    trackers().lock().unwrap().remove(&camera_uuid);
}

async fn fetch_base_setting(camera_uuid: Uuid) -> Result<BaseParameterSetting, String> {
    let value = radcam_commands::handle_control(CameraControl {
        camera_uuid,
        action: CameraAction::GetImageAdjustment,
    })
    .await
    .map_err(|error| format!("{error:?}"))?;
    serde_json::from_value(value).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(r: u8, g: u8, b: u8, manual: bool) -> BaseParameterSetting {
        BaseParameterSetting {
            awb_red: Some(r),
            awb_green: Some(g),
            awb_blue: Some(b),
            auto_awb: Some(if manual {
                BaseAutoWhiteBalanceModeValue::Manual
            } else {
                BaseAutoWhiteBalanceModeValue::Auto
            }),
            ..Default::default()
        }
    }

    #[test]
    fn manual_after_rgb_change_settles() {
        let start = base(10, 20, 30, false);
        let mut machine = SettleMachine::begin(&start);
        assert_eq!(
            machine.observe(&base(11, 20, 30, false)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&base(11, 20, 30, true)),
            ObserveResult::Settled
        );
    }

    #[test]
    fn already_manual_settles_on_first_rgb_change() {
        let start = base(10, 20, 30, true);
        let mut machine = SettleMachine::begin(&start);
        assert_eq!(
            machine.observe(&base(40, 20, 30, true)),
            ObserveResult::Settled
        );
    }

    #[test]
    fn auto_settles_on_rgb_stability() {
        let start = base(10, 20, 30, false);
        let mut machine = SettleMachine::begin(&start);
        assert_eq!(
            machine.observe(&base(40, 20, 30, false)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&base(40, 20, 30, false)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&base(40, 20, 30, false)),
            ObserveResult::Settled
        );
    }

    #[test]
    fn no_change_never_settles() {
        let start = base(10, 20, 30, true);
        let mut machine = SettleMachine::begin(&start);
        for _ in 0..5 {
            assert_eq!(
                machine.observe(&base(10, 20, 30, true)),
                ObserveResult::Continue
            );
        }
    }
}
