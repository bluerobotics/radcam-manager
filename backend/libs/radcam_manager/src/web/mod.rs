use std::net::SocketAddr;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::{ServiceExt, extract::Request};
use once_cell::sync::OnceCell;
use tokio::{signal, sync::Notify};
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tracing::*;

pub mod routes;

static RESTART_NOTIFY: OnceCell<Notify> = OnceCell::new();
static TERMINATE_NOTIFY: OnceCell<Notify> = OnceCell::new();
static SHUTDOWN_LISTENERS: Once = Once::new();
static RESTART_REQUESTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownReason {
    Signal,
    Restart,
}

pub fn request_restart() {
    info!("Backend restart requested");
    RESTART_REQUESTED.store(true, Ordering::SeqCst);
    RESTART_NOTIFY.get_or_init(Notify::new).notify_waiters();
}

pub async fn run(address: std::net::SocketAddr, default_api_version: u8) -> ShutdownReason {
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

async fn shutdown_signal() {
    SHUTDOWN_LISTENERS.call_once(|| {
        tokio::spawn(async {
            if signal::ctrl_c().await.is_ok() {
                TERMINATE_NOTIFY.get_or_init(Notify::new).notify_waiters();
            }
        });

        #[cfg(unix)]
        tokio::spawn(async {
            let mut signals = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler");

            if signals.recv().await.is_some() {
                TERMINATE_NOTIFY.get_or_init(Notify::new).notify_waiters();
            }
        });
    });

    tokio::select! {
        _ = TERMINATE_NOTIFY.get_or_init(Notify::new).notified() => {},
        _ = RESTART_NOTIFY.get_or_init(Notify::new).notified() => {},
    }
}
