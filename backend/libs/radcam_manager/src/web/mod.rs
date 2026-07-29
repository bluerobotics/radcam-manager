use std::net::SocketAddr;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::{ServiceExt, extract::Request};
use once_cell::sync::OnceCell;
use tokio::{signal, sync::Notify};
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tracing::*;

pub(crate) mod camera_state;
pub(crate) mod camera_ui;
pub(crate) mod control_bridge;
pub(crate) mod one_push_awb;
pub mod routes;
pub(crate) mod ws_connections;

static RESTART_NOTIFY: OnceCell<Notify> = OnceCell::new();
static TERMINATE_NOTIFY: OnceCell<Notify> = OnceCell::new();
static SHUTDOWN_LISTENERS: Once = Once::new();
static RESTART_REQUESTED: AtomicBool = AtomicBool::new(false);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownReason {
    Signal,
    Restart,
}

/// Returns true once a restart or terminate has been requested.
fn shutting_down() -> bool {
    SHUTTING_DOWN.load(Ordering::SeqCst)
}

/// Resolves when the web server is shutting down for restart or terminate.
pub async fn wait_shutdown() {
    let terminate = TERMINATE_NOTIFY.get_or_init(Notify::new).notified();
    let restart = RESTART_NOTIFY.get_or_init(Notify::new).notified();
    tokio::pin!(terminate, restart);
    // Register before re-checking so a store between the check and registration is not lost.
    terminate.as_mut().enable();
    restart.as_mut().enable();
    if shutting_down() {
        return;
    }
    tokio::select! {
        _ = terminate => {},
        _ = restart => {},
    }
}

pub fn request_restart() {
    info!("Backend restart requested");
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
    RESTART_REQUESTED.store(true, Ordering::SeqCst);
    RESTART_NOTIFY.get_or_init(Notify::new).notify_waiters();
}

pub async fn run(address: std::net::SocketAddr, default_api_version: u8) -> ShutdownReason {
    // A prior in-process restart left this set; clear it so wait_shutdown blocks again.
    prepare_run();

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
    let mut first = true;
    loop {
        if first {
            first = false;
        } else {
            interval.tick().await;
        }

        let listener = match tokio::net::TcpListener::bind(&address).await {
            Ok(listener) => listener,
            Err(error) => {
                error!("WebServer TCP bind error: {error}");
                continue;
            }
        };

        let app =
            NormalizePathLayer::trim_trailing_slash().layer(routes::router(default_api_version));
        let service = ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(app);

        info!("Running web server on address {address:?}");

        if let Err(error) = axum::serve(listener, service)
            .with_graceful_shutdown(shutdown_signal())
            .await
        {
            error!("WebServer error: {error}");
            continue;
        }

        if RESTART_REQUESTED.swap(false, Ordering::SeqCst) {
            return ShutdownReason::Restart;
        }

        return ShutdownReason::Signal;
    }
}

/// Clears sticky shutdown state left by a previous in-process restart.
fn prepare_run() {
    SHUTTING_DOWN.store(false, Ordering::SeqCst);
}

async fn shutdown_signal() {
    SHUTDOWN_LISTENERS.call_once(|| {
        tokio::spawn(async {
            if signal::ctrl_c().await.is_ok() {
                SHUTTING_DOWN.store(true, Ordering::SeqCst);
                TERMINATE_NOTIFY.get_or_init(Notify::new).notify_waiters();
            }
        });

        #[cfg(unix)]
        tokio::spawn(async {
            let mut signals = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler");

            if signals.recv().await.is_some() {
                SHUTTING_DOWN.store(true, Ordering::SeqCst);
                TERMINATE_NOTIFY.get_or_init(Notify::new).notify_waiters();
            }
        });
    });

    wait_shutdown().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_run_clears_shutting_down_after_restart_request() {
        request_restart();
        assert!(shutting_down());
        prepare_run();
        assert!(!shutting_down());
        // Drop the sticky restart flag so other tests / later run() see a clean slate.
        RESTART_REQUESTED.store(false, Ordering::SeqCst);
    }
}
