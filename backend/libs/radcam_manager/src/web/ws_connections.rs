use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use radcam_api::ConnectionStats;
use tracing::*;

const WINDOW_SECS: usize = 60;

static NEXT_CONNECTION_ID: AtomicU64 = AtomicU64::new(1);
static CONNECTIONS: OnceCell<Mutex<HashMap<ConnectionId, Entry>>> = OnceCell::new();

/// Opaque identifier for an active WebSocket connection.
pub(crate) type ConnectionId = u64;

struct Entry {
    connected_at: DateTime<Utc>,
    connected_instant: Instant,
    bandwidth: BandwidthWindow,
}

struct BandwidthWindow {
    upload_buckets: [u64; WINDOW_SECS],
    download_buckets: [u64; WINDOW_SECS],
    head: usize,
    last_advance: Instant,
}

impl BandwidthWindow {
    fn new() -> Self {
        Self {
            upload_buckets: [0; WINDOW_SECS],
            download_buckets: [0; WINDOW_SECS],
            head: 0,
            last_advance: Instant::now(),
        }
    }

    fn advance(&mut self) {
        let elapsed = self.last_advance.elapsed();
        let slots = elapsed.as_secs() as usize;
        if slots == 0 {
            return;
        }

        for _ in 0..slots.min(WINDOW_SECS) {
            self.head = (self.head + 1) % WINDOW_SECS;
            self.upload_buckets[self.head] = 0;
            self.download_buckets[self.head] = 0;
        }
        // Preserve sub-second remainder so buckets stay aligned to wall clock.
        self.last_advance += Duration::from_secs(slots as u64);
    }

    fn record_upload(&mut self, bytes: u64) {
        self.advance();
        self.upload_buckets[self.head] += bytes;
    }

    fn record_download(&mut self, bytes: u64) {
        self.advance();
        self.download_buckets[self.head] += bytes;
    }

    fn kbps_for_age(&self, age_secs: f64) -> (f64, f64) {
        let upload_bytes: u64 = self.upload_buckets.iter().sum();
        let download_bytes: u64 = self.download_buckets.iter().sum();
        let seconds = age_secs.min(WINDOW_SECS as f64).max(1.0);
        (
            bytes_to_kbps(upload_bytes, seconds),
            bytes_to_kbps(download_bytes, seconds),
        )
    }
}

/// Register a new WebSocket connection and return its id.
#[instrument(level = "debug")]
pub(crate) fn register() -> ConnectionId {
    let connection_id = NEXT_CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
    let mut lock = connections().lock().unwrap();
    lock.insert(
        connection_id,
        Entry {
            connected_at: Utc::now(),
            connected_instant: Instant::now(),
            bandwidth: BandwidthWindow::new(),
        },
    );
    connection_id
}

/// Forget a closed WebSocket connection.
#[instrument(level = "debug")]
pub(crate) fn unregister(connection_id: ConnectionId) {
    connections().lock().unwrap().remove(&connection_id);
}

/// Record bytes received from the client.
#[instrument(level = "debug", skip_all, fields(%connection_id, bytes))]
pub(crate) fn record_upload(connection_id: ConnectionId, bytes: u64) {
    if bytes == 0 {
        return;
    }

    let mut lock = connections().lock().unwrap();
    let Some(entry) = lock.get_mut(&connection_id) else {
        return;
    };
    entry.bandwidth.record_upload(bytes);
}

/// Record bytes sent to the client.
#[instrument(level = "debug", skip_all, fields(%connection_id, bytes))]
pub(crate) fn record_download(connection_id: ConnectionId, bytes: u64) {
    if bytes == 0 {
        return;
    }

    let mut lock = connections().lock().unwrap();
    let Some(entry) = lock.get_mut(&connection_id) else {
        return;
    };
    entry.bandwidth.record_download(bytes);
}

/// Snapshot stats for `connection_id`, advancing all windows first.
///
/// ponytail: each connection's 1 Hz tick advances every connection (O(n²) across
/// clients). Fine for <<100 clients; upgrade: advance lazily in `record_*` or keep
/// running totals.
#[instrument(level = "debug")]
pub(crate) fn snapshot(connection_id: ConnectionId) -> ConnectionStats {
    let now = Utc::now();
    let mut lock = connections().lock().unwrap();
    for entry in lock.values_mut() {
        entry.bandwidth.advance();
    }

    let clients_connected = lock.len();

    let (total_upload_kbps, total_download_kbps) = lock
        .values()
        .map(|entry| {
            let age_secs = entry.connected_instant.elapsed().as_secs_f64();
            entry.bandwidth.kbps_for_age(age_secs)
        })
        .fold(
            (0.0, 0.0),
            |(total_upload, total_download), (upload, download)| {
                (total_upload + upload, total_download + download)
            },
        );

    let Some(entry) = lock.get(&connection_id) else {
        return ConnectionStats {
            connected: false,
            since: now,
            clients_connected,
            this_upload_kbps: 0.0,
            this_download_kbps: 0.0,
            total_upload_kbps,
            total_download_kbps,
        };
    };

    let age_secs = entry.connected_instant.elapsed().as_secs_f64();
    let (this_upload_kbps, this_download_kbps) = entry.bandwidth.kbps_for_age(age_secs);

    ConnectionStats {
        connected: true,
        since: entry.connected_at,
        clients_connected,
        this_upload_kbps,
        this_download_kbps,
        total_upload_kbps,
        total_download_kbps,
    }
}

fn connections() -> &'static Mutex<HashMap<ConnectionId, Entry>> {
    CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn bytes_to_kbps(bytes: u64, seconds: f64) -> f64 {
    (bytes as f64 * 8.0) / 1000.0 / seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bandwidth_window_averages_recorded_bytes() {
        let mut window = BandwidthWindow::new();
        window.record_upload(1_000);
        window.record_download(2_000);
        let (upload, download) = window.kbps_for_age(WINDOW_SECS as f64);
        assert!((upload - (1_000.0 * 8.0 / 1000.0 / 60.0)).abs() < 1e-9);
        assert!((download - (2_000.0 * 8.0 / 1000.0 / 60.0)).abs() < 1e-9);
    }

    #[test]
    fn bandwidth_window_advance_preserves_remainder() {
        let mut window = BandwidthWindow::new();
        window.last_advance = Instant::now() - Duration::from_millis(1500);
        window.record_upload(100);
        assert!(window.last_advance.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn kbps_for_age_uses_shorter_window_early_on() {
        let mut window = BandwidthWindow::new();
        window.record_upload(1_000);
        let (upload, _) = window.kbps_for_age(1.0);
        assert!((upload - (1_000.0 * 8.0 / 1000.0)).abs() < 1e-9);
    }
}
