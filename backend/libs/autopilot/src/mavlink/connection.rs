use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::Result;
use mavlink::{AsyncMavConnection, MavHeader, ardupilotmega::MavMessage};
use tokio::sync::{Notify, RwLock, broadcast};
use tracing::*;

#[derive(Clone)]
pub struct Connection {
    address: String,
    inner: Arc<RwLock<Box<dyn AsyncMavConnection<MavMessage> + Sync + Send>>>,
    coordinator: Arc<ReconnectCoordinator>,
    sender: broadcast::Sender<Message>,
    reconnect_heartbeat: Arc<RwLock<Option<(MavHeader, MavMessage)>>>,
}

#[derive(Default)]
struct ReconnectCoordinator {
    notify: Notify,
    is_running: AtomicBool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Received((MavHeader, MavMessage)),
    ToBeSent((MavHeader, MavMessage)),
}

impl Connection {
    #[instrument(level = "debug")]
    pub async fn new(address: String) -> Self {
        let inner = Arc::new(RwLock::new(Self::connect(&address).await));

        let (sender, _receiver) = broadcast::channel::<Message>(10000);

        let coordinator = Arc::new(ReconnectCoordinator::default());

        Self {
            address,
            inner,
            coordinator,
            sender,
            reconnect_heartbeat: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_reconnect_heartbeat(&self, header: MavHeader, message: MavMessage) {
        *self.reconnect_heartbeat.write().await = Some((header, message));
    }

    #[instrument(level = "debug")]
    pub async fn connect(address: &str) -> Box<dyn AsyncMavConnection<MavMessage> + Sync + Send> {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            debug!("Connecting...");

            match mavlink::connect_async(address).await {
                Ok(connection) => {
                    info!("Successfully connected");
                    return connection;
                }
                Err(error) => {
                    error!("Failed to connect, trying again in one second. Reason: {error:?}.");
                }
            }
        }
    }

    pub async fn send(
        &mut self,
        header: &MavHeader,
        message: &MavMessage,
        timeout: Duration,
    ) -> Result<()> {
        tokio::time::timeout(timeout, self.send_inner(header, message))
            .await
            .map_err(anyhow::Error::msg)
    }

    async fn send_inner(&mut self, header: &MavHeader, message: &MavMessage) {
        loop {
            {
                let mavlink = self.inner.read().await;

                match mavlink.send(header, message).await {
                    Ok(_) => return,
                    Err(error) => {
                        error!("Failed sending message: {error:?}");
                    }
                }
            }

            self.reconnect().await;
        }
    }

    pub async fn recv(&mut self, timeout: Duration) -> (MavHeader, MavMessage) {
        loop {
            {
                let mavlink = self.inner.read().await;

                match tokio::time::timeout(timeout, mavlink.recv()).await {
                    Ok(Ok(inner)) => return inner,
                    Ok(Err(mavlink::error::MessageReadError::Io(error))) => {
                        error!("Failed receiving message: {error:?}");
                    }
                    Ok(Err(mavlink::error::MessageReadError::Parse(error))) => {
                        warn!("Failed receiving message: {error:?}");
                        continue;
                    }
                    Err(_) => {
                        error!("Timeout while receiving message...");
                    }
                }
            }

            self.reconnect().await;
        }
    }

    pub async fn reconnect(&mut self) {
        let coordinator = self.coordinator.clone();

        if coordinator.is_running.swap(true, Ordering::AcqRel) {
            coordinator.notify.notified().await;
            return;
        }

        *self.inner.write().await = Self::connect(&self.address).await;

        if let Some((header, message)) = self.reconnect_heartbeat.read().await.as_ref() {
            let conn = self.inner.read().await;
            match conn.send(header, message).await {
                Ok(_) => {
                    info!("Post-reconnect heartbeat sent (conntrack + camera re-registration)")
                }
                Err(error) => warn!("Failed to send post-reconnect heartbeat: {error:?}"),
            }
        }

        coordinator.is_running.store(false, Ordering::Release);
        coordinator.notify.notify_waiters();
    }

    pub fn get_sender(&self) -> broadcast::Sender<Message> {
        self.sender.clone()
    }

    pub fn get_receiver(&self) -> broadcast::Receiver<Message> {
        self.sender.subscribe()
    }
}
