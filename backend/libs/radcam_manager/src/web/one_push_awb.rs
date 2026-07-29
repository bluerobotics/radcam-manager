//! Backend-owned one-push white balance lifecycle.
//!
//! Pure settle logic is unit-tested; the tracker polls base parameters and drives
//! [`camera_ui`] / [`camera_state`] so every client stays in sync.

use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use once_cell::sync::OnceCell;
use radcam_api::{CameraStateEvent, OnePushAwbPhase, OnePushAwbStatus};
use radcam_commands::{Action as CameraAction, CameraControl};
use tokio::task::JoinHandle;
use tracing::*;
use uuid::Uuid;

use crate::web::{camera_state, camera_ui};

/// Consecutive identical Manual RGB samples required after movement before settle.
pub const STABLE_SAMPLES: u8 = 2;
/// Wall-clock ceiling while waiting for Manual RGB to move and hold steady.
pub const RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
/// Minimum time WB controls stay busy from trigger (Cooldown fills any shortfall).
pub const MIN_BUSY: std::time::Duration = std::time::Duration::from_secs(1);
/// Poll interval while Running.
pub const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// `auto_awb` value for Manual in the camera protocol (`BaseAutoWhiteBalanceModeValue::Manual`).
const AUTO_AWB_MANUAL: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbSample {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

/// WB fields pulled from a getImageAdjustment JSON body (ignores unrelated enums).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AwbSample {
    pub rgb: Option<RgbSample>,
    pub auto_awb: Option<u8>,
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
    generation: u64,
}

static TRACKERS: OnceCell<Mutex<HashMap<Uuid, TrackerEntry>>> = OnceCell::new();
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

impl AwbSample {
    /// Read only AWB fields so unknown/extra camera enums cannot abort the tracker.
    pub fn from_json(value: &serde_json::Value) -> Self {
        let u8_field = |name: &str| value.get(name).and_then(|v| v.as_u64()).map(|v| v as u8);
        let red = u8_field("awb_red");
        let green = u8_field("awb_green");
        let blue = u8_field("awb_blue");
        let rgb = match (red, green, blue) {
            (Some(red), Some(green), Some(blue)) => Some(RgbSample { red, green, blue }),
            _ => None,
        };
        Self {
            rgb,
            auto_awb: u8_field("auto_awb"),
        }
    }
}

impl SettleMachine {
    pub fn begin(start: &AwbSample) -> Self {
        Self {
            start_rgb: start.rgb,
            last_rgb: start.rgb,
            saw_change: false,
            stable_count: 0,
        }
    }

    pub fn saw_change(&self) -> bool {
        self.saw_change
    }

    /// Feed a base-parameter sample.
    ///
    /// Settles only after RGB has moved and then held steady while Manual — never on
    /// Manual alone or an unchanged Manual snapshot (onceAWB often lags before gains move).
    pub fn observe(&mut self, sample: &AwbSample) -> ObserveResult {
        let manual = sample.auto_awb == Some(AUTO_AWB_MANUAL);

        if let Some(current) = sample.rgb {
            if let Some(last) = self.last_rgb {
                if current != last {
                    self.saw_change = true;
                    self.stable_count = 0;
                } else if self.saw_change && manual {
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

        if self.saw_change && manual && self.stable_count >= STABLE_SAMPLES {
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

    let generation = NEXT_GENERATION.fetch_add(1, Ordering::Relaxed);

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

    let handle = tokio::spawn(run_tracker(camera_uuid, generation).instrument(Span::current()));
    trackers()
        .lock()
        .unwrap()
        .insert(camera_uuid, TrackerEntry { handle, generation });
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

fn is_current_generation(camera_uuid: Uuid, generation: u64) -> bool {
    trackers()
        .lock()
        .unwrap()
        .get(&camera_uuid)
        .is_some_and(|entry| entry.generation == generation)
}

#[instrument(level = "debug", skip_all, fields(%camera_uuid, generation))]
async fn run_tracker(camera_uuid: Uuid, generation: u64) {
    let start = Instant::now();

    // Retry the opening snapshot — a single failed GET right after onceAWB used to
    // finish_timeout immediately and clear the button in well under a second.
    let (initial_json, initial) = loop {
        match fetch_base_json(camera_uuid).await {
            Ok(value) => {
                let sample = AwbSample::from_json(&value);
                break (value, sample);
            }
            Err(error) => {
                warn!("one-push AWB start snapshot failed: {error}");
                if !is_current_generation(camera_uuid, generation) {
                    return;
                }
                if start.elapsed() >= RUN_TIMEOUT {
                    finish_timeout(camera_uuid, generation);
                    return;
                }
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        }
    };

    if !is_current_generation(camera_uuid, generation) {
        return;
    }

    camera_state::emit(CameraStateEvent {
        camera_uuid,
        base_parameters: Some(initial_json),
        ..Default::default()
    });

    let mut machine = SettleMachine::begin(&initial);
    let mut last_emitted = initial.clone();

    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        if !is_current_generation(camera_uuid, generation) {
            return;
        }

        let sample_json = match fetch_base_json(camera_uuid).await {
            Ok(value) => value,
            Err(error) => {
                warn!("one-push AWB poll failed: {error}");
                if start.elapsed() >= RUN_TIMEOUT {
                    if machine.saw_change() {
                        finish_timeout(camera_uuid, generation);
                    } else {
                        finish_settled(camera_uuid, start, generation).await;
                    }
                    return;
                }
                continue;
            }
        };
        let sample = AwbSample::from_json(&sample_json);

        if sample != last_emitted {
            camera_state::emit(CameraStateEvent {
                camera_uuid,
                base_parameters: Some(sample_json),
                ..Default::default()
            });
            last_emitted = sample.clone();
        }

        // Do not quiet-settle on unchanged Manual mid-run: onceAWB often leaves gains
        // untouched for several seconds before they move.
        if machine.observe(&sample) == ObserveResult::Settled {
            finish_settled(camera_uuid, start, generation).await;
            return;
        }

        if start.elapsed() >= RUN_TIMEOUT {
            if machine.saw_change() {
                finish_timeout(camera_uuid, generation);
            } else {
                finish_settled(camera_uuid, start, generation).await;
            }
            return;
        }
    }
}

/// Hold Cooldown only long enough to honour [`MIN_BUSY`] from `started_at`.
async fn finish_settled(camera_uuid: Uuid, started_at: Instant, generation: u64) {
    if !is_current_generation(camera_uuid, generation) {
        return;
    }
    let remaining = MIN_BUSY.saturating_sub(started_at.elapsed());
    if !remaining.is_zero() {
        camera_ui::set_one_push_awb(
            camera_uuid,
            Some(OnePushAwbStatus {
                phase: OnePushAwbPhase::Cooldown,
            }),
        );
        tokio::time::sleep(remaining).await;
        if !is_current_generation(camera_uuid, generation) {
            return;
        }
    }
    camera_ui::set_one_push_awb(camera_uuid, None);
    remove_tracker_if_current(camera_uuid, generation);
}

fn finish_timeout(camera_uuid: Uuid, generation: u64) {
    if !is_current_generation(camera_uuid, generation) {
        return;
    }
    camera_ui::set_warning(camera_uuid, "One-push white balance timed out".to_string());
    camera_ui::set_one_push_awb(camera_uuid, None);
    remove_tracker_if_current(camera_uuid, generation);
}

fn remove_tracker_if_current(camera_uuid: Uuid, generation: u64) {
    let mut lock = trackers().lock().unwrap();
    if lock
        .get(&camera_uuid)
        .is_some_and(|entry| entry.generation == generation)
    {
        lock.remove(&camera_uuid);
    }
}

async fn fetch_base_json(camera_uuid: Uuid) -> Result<serde_json::Value, String> {
    radcam_commands::handle_control(CameraControl {
        camera_uuid,
        action: CameraAction::GetImageAdjustment,
    })
    .await
    .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(r: u8, g: u8, b: u8, manual: bool) -> AwbSample {
        AwbSample {
            rgb: Some(RgbSample {
                red: r,
                green: g,
                blue: b,
            }),
            auto_awb: Some(if manual { AUTO_AWB_MANUAL } else { 0 }),
        }
    }

    #[test]
    fn awb_sample_ignores_unrelated_fields() {
        let value = serde_json::json!({
            "sceneMode": 0,
            "auto_awb": 1,
            "awb_red": 10,
            "awb_green": 20,
            "awb_blue": 30,
            "extra_future_field": true,
        });
        assert_eq!(AwbSample::from_json(&value), sample(10, 20, 30, true));
    }

    #[test]
    fn manual_after_rgb_change_needs_stable_samples() {
        let start = sample(10, 20, 30, false);
        let mut machine = SettleMachine::begin(&start);
        assert_eq!(
            machine.observe(&sample(11, 20, 30, false)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&sample(11, 20, 30, true)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&sample(11, 20, 30, true)),
            ObserveResult::Settled
        );
    }

    #[test]
    fn already_manual_needs_stable_rgb_after_change() {
        let start = sample(10, 20, 30, true);
        let mut machine = SettleMachine::begin(&start);
        assert_eq!(
            machine.observe(&sample(40, 20, 30, true)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&sample(40, 20, 30, true)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&sample(40, 20, 30, true)),
            ObserveResult::Settled
        );
    }

    #[test]
    fn unchanged_manual_does_not_settle_early() {
        // Repro: Manual gains set, onceAWB triggered — camera lags before RGB moves.
        let start = sample(41, 128, 81, true);
        let mut machine = SettleMachine::begin(&start);
        for _ in 0..5 {
            assert_eq!(
                machine.observe(&sample(41, 128, 81, true)),
                ObserveResult::Continue
            );
        }
        assert!(!machine.saw_change());
    }

    #[test]
    fn auto_rgb_stability_waits_for_manual_then_hold() {
        let start = sample(10, 20, 30, false);
        let mut machine = SettleMachine::begin(&start);
        assert_eq!(
            machine.observe(&sample(40, 20, 30, false)),
            ObserveResult::Continue
        );
        // Auto identical samples do not accumulate Manual stability.
        assert_eq!(
            machine.observe(&sample(40, 20, 30, false)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&sample(40, 20, 30, true)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&sample(40, 20, 30, true)),
            ObserveResult::Settled
        );
    }

    #[test]
    fn mid_hunt_rgb_change_resets_stability() {
        let start = sample(41, 128, 81, true);
        let mut machine = SettleMachine::begin(&start);
        assert_eq!(
            machine.observe(&sample(50, 128, 90, true)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&sample(50, 128, 90, true)),
            ObserveResult::Continue
        );
        // Still hunting — stability resets.
        assert_eq!(
            machine.observe(&sample(55, 128, 95, true)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&sample(55, 128, 95, true)),
            ObserveResult::Continue
        );
        assert_eq!(
            machine.observe(&sample(55, 128, 95, true)),
            ObserveResult::Settled
        );
    }

    #[test]
    fn min_busy_remaining_is_zero_after_floor() {
        assert_eq!(MIN_BUSY.saturating_sub(MIN_BUSY), std::time::Duration::ZERO);
        assert_eq!(
            MIN_BUSY.saturating_sub(MIN_BUSY + std::time::Duration::from_millis(1)),
            std::time::Duration::ZERO
        );
        assert_eq!(
            MIN_BUSY.saturating_sub(std::time::Duration::from_millis(250)),
            std::time::Duration::from_millis(750)
        );
    }
}
