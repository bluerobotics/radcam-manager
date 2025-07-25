mod connection;
pub mod parameters;

use std::sync::Arc;

use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use mavlink::{
    self, MavHeader, MessageData,
    ardupilotmega::{CAMERA_SETTINGS_DATA, COMMAND_LONG_DATA, MavCmd, MavMessage, MavResult},
};
use tokio::sync::{RwLock, broadcast};
use tracing::*;

use crate::{
    mavlink::{
        connection::{Connection, Message},
        parameters::ParamEncodingType,
    },
    parameters::{ParamType, Parameter},
};

#[derive(Debug)]
pub struct MavlinkComponent {
    inner: Arc<RwLock<ComponentInner>>,
    sender_task_handle: tokio::task::JoinHandle<()>,
    receiver_task_handle: tokio::task::JoinHandle<()>,
    params_sync_task_handle: tokio::task::JoinHandle<()>,
    heartbeat_task_handle: tokio::task::JoinHandle<()>,
}

impl MavlinkComponent {
    #[instrument(level = "debug")]
    pub async fn new(address: String, system_id: u8, component_id: u8) -> Self {
        let inner = Arc::new(RwLock::new(
            ComponentInner::new(address, system_id, component_id).await,
        ));

        let sender_task_handle = tokio::spawn(Self::sender_task(inner.clone()));
        let receiver_task_handle = tokio::spawn(Self::receiver_task(inner.clone()));
        let heartbeat_task_handle = tokio::spawn(Self::heartbeat_task(inner.clone()));

        Self::configure_parameter_encoding(inner.clone()).await;
        Self::update_all_params(inner.clone()).await;

        let params_sync_task_handle = tokio::spawn(Self::params_sync_task(inner.clone()));

        Self {
            inner,
            sender_task_handle,
            receiver_task_handle,
            params_sync_task_handle,
            heartbeat_task_handle,
        }
    }

    #[instrument(level = "debug", skip(inner))]
    async fn sender_task(inner: Arc<RwLock<ComponentInner>>) {
        let mut receiver;
        let timeout = std::time::Duration::from_secs(10);

        {
            let inner_guard = inner.read().await;
            receiver = inner_guard.get_receiver().await;
        }

        loop {
            // Receive messages from the local components
            let (header, message) = match receiver.recv().await {
                Ok(Message::ToBeSent(inner)) => inner,
                Ok(Message::Received(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    unreachable!(
                        "Closed channel: This should never happen, this channel is owned by ComponentInner!"
                    );
                }
                Err(broadcast::error::RecvError::Lagged(samples)) => {
                    warn!(
                        "Channel is lagged behind by {samples} messages. Expect degraded performance on the mavlink responsiviness."
                    );

                    continue;
                }
            };

            // Send the response from the local components to the Mavlink network
            if let Err(error) = inner
                .write()
                .await
                .connection
                .send(&header, &message, timeout)
                .await
            {
                error!("Failed sending message to Mavlink Connection: {error:?}");

                continue;
            }
        }
    }

    #[instrument(level = "debug", skip(inner))]
    async fn receiver_task(inner: Arc<RwLock<ComponentInner>>) {
        let sender;
        let timeout = std::time::Duration::from_millis(500);

        {
            let inner_guard = inner.read().await;
            sender = inner_guard.get_sender().await;
        }

        loop {
            // Receive from the Mavlink network
            let (header, message) = match inner.write().await.connection.recv(timeout).await {
                Ok(inner) => inner,
                Err(error) => {
                    warn!("Failed receiving mavlink message: {error:?}");
                    continue;
                }
            };

            // Send the received message to the components
            if let Err(error) = sender.send(Message::Received((header, message))) {
                warn!("Failed receiving mavlink message: {error:?}");

                continue;
            }
        }
    }

    #[instrument(level = "debug", skip(inner))]
    async fn heartbeat_task(inner: Arc<RwLock<ComponentInner>>) {
        let sender;
        let system_id;
        let component_id;

        {
            let inner_guard = inner.read().await;

            sender = inner_guard.get_sender().await;
            system_id = inner_guard.system_id;
            component_id = inner_guard.component_id;
        }

        let mut header = MavHeader {
            system_id,
            component_id,
            sequence: 0,
        };

        let message = MavMessage::HEARTBEAT(mavlink::ardupilotmega::HEARTBEAT_DATA {
            custom_mode: 0,
            mavtype: mavlink::ardupilotmega::MavType::MAV_TYPE_CAMERA,
            autopilot: mavlink::ardupilotmega::MavAutopilot::MAV_AUTOPILOT_INVALID,
            base_mode: mavlink::ardupilotmega::MavModeFlag::empty(),
            system_status: mavlink::ardupilotmega::MavState::MAV_STATE_STANDBY,
            mavlink_version: 0x3,
        });

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            if let Err(error) = sender.send(Message::ToBeSent((header, message.clone()))) {
                warn!("Failed sending message: {error:?}");
                continue;
            }

            header.sequence = header.sequence.wrapping_add(1);
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn enable_lua_script(&self, overwrite: bool) -> Result<bool> {
        let mut autopilot_reboot_required = overwrite;

        let encoding = self.encoding().await;

        let mut param = self.get_param("SCR_ENABLE", false).await?;
        let old_value = param.value;
        param.value.set_value(ParamType::REAL32(1.0), encoding)?;
        let new_value = param.value;

        if overwrite || old_value != new_value {
            self.set_param(param).await?;
            autopilot_reboot_required = true;
        }

        Ok(autopilot_reboot_required)
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn restart_autopilot(&self) -> Result<()> {
        let target_system = { self.inner.read().await.system_id };
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;

        self.send_command(COMMAND_LONG_DATA {
            target_system,
            target_component,
            confirmation: 0,
            command: MavCmd::MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN,
            param1: 1.0, // autopilot
            ..Default::default()
        })
        .await
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn send_command(&self, mut command: COMMAND_LONG_DATA) -> Result<()> {
        let target_system;
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let this_system;
        let this_component;
        let sender;
        let mut receiver;

        {
            let inner_guard = self.inner.read().await;

            target_system = inner_guard.system_id;
            this_system = inner_guard.system_id;
            this_component = inner_guard.component_id;
            sender = inner_guard.get_sender().await;
            receiver = inner_guard.get_receiver().await;
        }

        let header = MavHeader {
            system_id: this_system,
            component_id: this_component,
            sequence: 0,
        };
        let message = MavMessage::COMMAND_LONG(command.clone());

        let max_retries = 5;
        while command.confirmation < max_retries {
            debug!("Sent command {:?}", command.command);
            sender.send(Message::ToBeSent((header, message.clone())))?;
            command.confirmation += 1;

            let wait_command_ack = async {
                loop {
                    use broadcast::error::RecvError;

                    match receiver.recv().await {
                        Ok(Message::Received((recv_header, recv_message)))
                            if recv_header.system_id == target_system
                                && recv_header.component_id == target_component
                                && matches!(recv_message, MavMessage::COMMAND_ACK(_)) =>
                        {
                            if let MavMessage::COMMAND_ACK(command_ack) = recv_message {
                                if command_ack.command == command.command {
                                    return match command_ack.result {
                                        MavResult::MAV_RESULT_ACCEPTED => Ok(()),
                                        reason => Err(anyhow!(
                                            "Command {:?} rejected: {reason:?}",
                                            command_ack.command
                                        )),
                                    };
                                }
                            }
                        }
                        Ok(_) => continue,
                        Err(RecvError::Closed) => {
                            return Err(anyhow!("Receiver channel closed"));
                        }
                        Err(RecvError::Lagged(n)) => {
                            warn!("Receiver lagged by {n} messages");
                            continue;
                        }
                    }
                }
            };

            match tokio::time::timeout(tokio::time::Duration::from_secs(5), wait_command_ack).await
            {
                Ok(res) => return res,
                Err(_) => {
                    warn!("Timeout for command {:?}, retrying", command.command);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }

        Ok(())
    }

    pub async fn request_camera_settings(&self) -> Result<CAMERA_SETTINGS_DATA> {
        let target_system = { self.inner.read().await.system_id };
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;

        self.send_command(COMMAND_LONG_DATA {
            command: MavCmd::MAV_CMD_REQUEST_MESSAGE,
            target_system,
            target_component,
            confirmation: 0,
            param1: CAMERA_SETTINGS_DATA::ID as f32,
            ..Default::default()
        })
        .await?;

        self.wait_camera_settings().await
    }

    pub async fn wait_camera_settings(&self) -> Result<CAMERA_SETTINGS_DATA> {
        let target_system;
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let mut receiver;

        {
            let inner_guard = self.inner.read().await;

            target_system = inner_guard.system_id;
            receiver = inner_guard.get_receiver().await;
        }

        let wait_message = async {
            loop {
                use broadcast::error::RecvError;

                match receiver.recv().await {
                    Ok(Message::Received((recv_header, recv_message)))
                        if recv_header.system_id == target_system
                            && recv_header.component_id == target_component
                            && matches!(recv_message, MavMessage::COMMAND_ACK(_)) =>
                    {
                        if let MavMessage::CAMERA_SETTINGS(camera_settings) = recv_message {
                            return Ok(camera_settings);
                        }
                    }
                    Ok(_) => continue,
                    Err(RecvError::Closed) => {
                        return Err(anyhow!("Receiver channel closed"));
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Receiver lagged by {n} messages");
                        continue;
                    }
                }
            }
        };

        match tokio::time::timeout(tokio::time::Duration::from_secs(5), wait_message).await {
            Ok(res) => res,
            Err(_) => Err(anyhow!("Timeout waiting")),
        }
    }
}

impl Drop for MavlinkComponent {
    fn drop(&mut self) {
        self.sender_task_handle.abort();
        self.receiver_task_handle.abort();
        self.heartbeat_task_handle.abort();
        self.params_sync_task_handle.abort();
    }
}

pub(crate) struct ComponentInner {
    pub system_id: u8,
    pub component_id: u8,
    pub encoding: ParamEncodingType,
    pub parameters: IndexMap<String, Parameter>,
    connection: Connection,
}

impl std::fmt::Debug for ComponentInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentInner")
            .field("system_id", &self.system_id)
            .field("component_id", &self.component_id)
            .finish()
    }
}

impl ComponentInner {
    #[instrument(level = "debug")]
    pub async fn new(address: String, system_id: u8, component_id: u8) -> Self {
        Self {
            system_id,
            component_id,
            encoding: ParamEncodingType::default(),
            parameters: IndexMap::with_capacity(2048),
            connection: Connection::new(address).await,
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn get_sender(&self) -> broadcast::Sender<Message> {
        self.connection.get_sender()
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn get_receiver(&self) -> broadcast::Receiver<Message> {
        self.connection.get_receiver()
    }
}
