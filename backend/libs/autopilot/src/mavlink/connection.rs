use std::time::Duration;

use anyhow::Result;
use mavlink::{AsyncMavConnection, MavHeader, ardupilotmega::MavMessage};
use tokio::sync::broadcast;
use tracing::*;

pub struct Connection {
    address: String,
    inner: Option<Box<dyn AsyncMavConnection<MavMessage> + Sync + Send>>,
    sender: broadcast::Sender<(MavHeader, MavMessage)>,
}

impl Connection {
    #[instrument(level = "debug")]
    pub async fn new(address: String) -> Self {
        let inner = Some(Self::connect(&address).await);

        let (sender, _receiver) = broadcast::channel::<(MavHeader, MavMessage)>(2048);

        Self {
            address,
            inner,
            sender,
        }
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
            loop {
                let Some(mavlink) = &self.inner else {
                    break; // Break to trigger reconnection
                };

                if let Err(error) = mavlink.send(header, message).await {
                    error!("Failed sending message: {error:?}");

                    break; // Break to trigger reconnection
                }

                return;
            }

            {
                self.reconnect().await;
            }
        }
    }

    pub async fn recv(&mut self, timeout: Duration) -> Result<(MavHeader, MavMessage)> {
        tokio::time::timeout(timeout, self.recv_inner())
            .await
            .map_err(anyhow::Error::msg)
    }

    async fn recv_inner(&mut self) -> (MavHeader, MavMessage) {
        loop {
            loop {
                let Some(mavlink) = &self.inner else {
                    break; // Break to trigger reconnection
                };

                match mavlink.recv().await {
                    Ok(inner) => return inner,
                    Err(mavlink::error::MessageReadError::Io(error)) => {
                        error!("Failed receiving message: {error:?}");

                        break; // Break to trigger reconnection
                    }
                    Err(mavlink::error::MessageReadError::Parse(error)) => {
                        warn!("Failed receiving message: {error:?}");

                        continue;
                    }
                };
            }

            {
                self.reconnect().await;
            }
        }
    }

    pub async fn reconnect(&mut self) {
        self.inner.replace(Connection::connect(&self.address).await);
    }

    pub fn get_sender(&self) -> broadcast::Sender<(MavHeader, MavMessage)> {
        self.sender.clone()
    }

    pub fn get_receiver(&self) -> broadcast::Receiver<(MavHeader, MavMessage)> {
        self.sender.subscribe()
    }
}
