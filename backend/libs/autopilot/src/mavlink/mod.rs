mod connection;

use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use mavlink::{
    self, MavHeader, MessageData,
    ardupilotmega::{
        CAMERA_SETTINGS_DATA, COMMAND_LONG_DATA, MavCmd, MavMessage, MavProtocolCapability,
        MavResult,
    },
};
use tokio::sync::{RwLock, broadcast};
use tracing::*;

use crate::{
    mavlink::connection::{Connection, Message},
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

#[derive(Debug, Default, Clone, Copy)]
pub enum ParamEncodingType {
    CCast,
    ByteWise,
    #[default]
    Unsupported,
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

    #[instrument(level = "debug", skip(inner))]
    async fn configure_parameter_encoding(inner: Arc<RwLock<ComponentInner>>) {
        let target_system;
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let this_system;
        let this_component;
        let sender;
        let mut receiver;

        {
            let inner_guard = inner.read().await;

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
        let message = MavMessage::AUTOPILOT_VERSION_REQUEST(
            mavlink::ardupilotmega::AUTOPILOT_VERSION_REQUEST_DATA {
                target_system,
                target_component,
            },
        );

        debug!("Getting parameter encoding from target {target_system}:{target_component}...");

        let encoding = loop {
            debug!("Requesting Autopilot Version...");

            if let Err(error) = sender.send(Message::ToBeSent((header, message.clone()))) {
                warn!("Failed requesting parameter: {error:?}");

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }

            let receive = async {
                loop {
                    let (_header, message) = match receiver.recv().await {
                        Ok(Message::Received(inner)) => inner,
                        Ok(Message::ToBeSent(_)) => continue,
                        Err(error) => {
                            error!("Failed receiving requested paramter: {error:?}");

                            continue;
                        }
                    };

                    let MavMessage::AUTOPILOT_VERSION(data) = message else {
                        continue;
                    };

                    break data;
                }
            };

            let Ok(data) =
                tokio::time::timeout(tokio::time::Duration::from_secs(10), receive).await
            else {
                continue;
            };

            let encoding_c_cast = data
                .capabilities
                .contains(MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_PARAM_FLOAT)
                || data
                    .capabilities
                    .contains(MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_PARAM_ENCODE_C_CAST);
            let encoding_bytewise = data
                .capabilities
                .contains(MavProtocolCapability::MAV_PROTOCOL_CAPABILITY_PARAM_ENCODE_BYTEWISE);

            match (encoding_c_cast, encoding_bytewise) {
                (true, true) => {
                    warn!(
                        "Unexpected value: Both C_CAST and BYTEWISE encodings are set by the Autopilot. Choosing BYTEWISE, then."
                    );
                    break ParamEncodingType::ByteWise;
                }
                (true, false) => {
                    break ParamEncodingType::CCast;
                }
                (false, true) => {
                    break ParamEncodingType::ByteWise;
                }
                (false, false) => {
                    error!(
                        "Unexpected value: None of the C_CAST and BYTEWISE encodings are set by the Autopilot. Assuming C_CAST, then."
                    );
                    break ParamEncodingType::Unsupported;
                }
            }
        };

        debug!("Using parameter encoding {encoding:?}");
        inner.write().await.encoding = encoding;
    }

    #[instrument(level = "debug", skip(inner))]
    pub async fn update_all_params(inner: Arc<RwLock<ComponentInner>>) {
        let target_system;
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let this_system;
        let this_component;
        let sender;
        let mut receiver;
        let encoding;

        {
            let inner_guard = inner.read().await;

            target_system = inner_guard.system_id;
            this_system = inner_guard.system_id;
            this_component = inner_guard.component_id;
            sender = inner_guard.get_sender().await;
            receiver = inner_guard.get_receiver().await;
            encoding = inner_guard.encoding;
        }

        let header = MavHeader {
            system_id: this_system,
            component_id: this_component,
            sequence: 0,
        };
        let message =
            MavMessage::PARAM_REQUEST_LIST(mavlink::ardupilotmega::PARAM_REQUEST_LIST_DATA {
                target_system,
                target_component,
            });

        debug!("Getting parameter list from target {target_system}:{target_component}...");

        let mut params_to_refetch = Vec::with_capacity(2048);
        let mut parameters = HashMap::with_capacity(2048);

        'send: loop {
            let mut current_param = 0;

            debug!("Requesting parameter list...");

            if let Err(error) = sender.send(Message::ToBeSent((header, message.clone()))) {
                warn!("Failed requesting parameter: {error:?}");

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }

            loop {
                let (_header, message) = match receiver.recv().await {
                    Ok(Message::Received(inner)) => inner,
                    Ok(Message::ToBeSent(_)) => continue,
                    Err(error) => {
                        error!("Failed receiving requested parameter: {error:?}");

                        continue;
                    }
                };

                let MavMessage::PARAM_VALUE(data) = message else {
                    continue;
                };

                if data.param_index == u16::MAX {
                    // Skipping unrelated parameters

                    continue;
                }

                current_param += 1;
                if data.param_index + 1 != current_param {
                    params_to_refetch.push(data.param_index)
                }

                let parameter = match Parameter::try_new(&data, encoding) {
                    Ok(parameter) => parameter,
                    Err(error) => {
                        warn!("Failed creating parameter from {data:?}: {error:?}");

                        continue;
                    }
                };

                debug!(
                    "Received param [{}/{}] {parameter:?}...",
                    data.param_index + 1,
                    data.param_count
                );

                parameters.insert(parameter.name.clone(), parameter.clone());

                if (data.param_index + 1) == data.param_count {
                    if parameters.len() == data.param_count as usize {
                        debug!("Received all {:?} parameters", parameters.len());
                        break 'send;
                    }

                    debug!(
                        "Received {:?} parameters, but missed {:?}: {:?}. Retrying...",
                        parameters.len(),
                        params_to_refetch.len(),
                        params_to_refetch
                    );
                    continue 'send;
                }
            }
        }

        inner.write().await.parameters = parameters;
    }

    #[instrument(level = "debug", skip(inner))]
    async fn params_sync_task(inner: Arc<RwLock<ComponentInner>>) {
        let mut receiver;
        let encoding;

        {
            let inner_guard = inner.read().await;

            receiver = inner_guard.get_receiver().await;
            encoding = inner_guard.encoding;
        }

        loop {
            let (_header, message) = match receiver.recv().await {
                Ok(Message::Received(inner)) => inner,
                Ok(Message::ToBeSent(_)) => continue,
                Err(error) => {
                    warn!("Failed receiving message: {error:?}");
                    continue;
                }
            };

            let MavMessage::PARAM_VALUE(data) = message else {
                continue;
            };

            let parameter = match Parameter::try_new(&data, encoding) {
                Ok(parameter) => parameter,
                Err(error) => {
                    warn!("Failed creating parameter from {data:?}: {error:?}");

                    continue;
                }
            };

            inner
                .write()
                .await
                .parameters
                .entry(parameter.name.clone())
                .and_modify(|v| {
                    if v.value != parameter.value {
                        debug!(
                            "Parameter {:?} updated from {:?} to {:?}",
                            v.name, v.value, parameter.value,
                        );
                    }
                    *v = parameter.clone()
                })
                .or_insert_with(|| {
                    debug!("New parameter added: {parameter:?}");
                    parameter
                });
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn get_param(&self, param_name: &str, skip_cache: bool) -> Result<Parameter> {
        Self::get_param_inner(self.inner.clone(), param_name, skip_cache).await
    }

    #[instrument(level = "debug", skip(inner))]
    async fn get_param_inner(
        inner: Arc<RwLock<ComponentInner>>,
        param_name: &str,
        skip_cache: bool,
    ) -> Result<Parameter> {
        let target_system;
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let this_system;
        let this_component;
        let sender;
        let mut receiver;
        let encoding;

        {
            let inner_guard = inner.read().await;

            if !skip_cache {
                if let Some(parameter) = inner_guard.parameters.get(param_name) {
                    trace!("Got parameter from cache!");
                    return Ok(parameter.clone());
                }
            }

            target_system = inner_guard.system_id;
            this_system = inner_guard.system_id;
            this_component = inner_guard.component_id;
            sender = inner_guard.get_sender().await;
            receiver = inner_guard.get_receiver().await;
            encoding = inner_guard.encoding;
        }

        let header = MavHeader {
            system_id: this_system,
            component_id: this_component,
            sequence: 0,
        };
        let message =
            MavMessage::PARAM_REQUEST_READ(mavlink::ardupilotmega::PARAM_REQUEST_READ_DATA {
                param_index: -1,
                target_system,
                target_component,
                param_id: Parameter::param_name_to_id(param_name),
            });

        'sender: loop {
            if let Err(error) = sender.send(Message::ToBeSent((header, message.clone()))) {
                warn!("Failed requesting parameter: {error:?}");

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }

            loop {
                let (_header, message) = match receiver.recv().await {
                    Ok(Message::Received(inner)) => inner,
                    Ok(Message::ToBeSent(_)) => continue,
                    Err(error) => {
                        warn!("Failed receiving requested parameter: {error:?}");

                        continue 'sender;
                    }
                };

                let MavMessage::PARAM_VALUE(data) = message else {
                    continue;
                };

                let parameter = match Parameter::try_new(&data, encoding) {
                    Ok(parameter) => parameter,
                    Err(error) => {
                        warn!("Failed creating parameter from {data:?}: {error:?}");

                        continue;
                    }
                };

                debug!(
                    "Received param [{}/{}] {parameter:?}...",
                    data.param_index.saturating_add(1),
                    data.param_count
                );

                inner
                    .write()
                    .await
                    .parameters
                    .insert(param_name.to_string(), parameter.clone());

                return Ok(parameter);
            }
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn enable_lua_script(&self) -> Result<()> {
        let encoding = self.encoding().await;

        let mut param = self.get_param("SCR_ENABLE", false).await?;
        param.value.set_value(ParamType::REAL32(1.0), encoding)?;

        self.set_param(param).await?;

        Ok(())
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
            Ok(res) => return res,
            Err(_) => return Err(anyhow!("Timeout waiting")),
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn set_param(&self, parameter: Parameter) -> Result<Parameter> {
        Self::set_param_inner(self.inner.clone(), parameter).await
    }

    #[instrument(level = "debug", skip(inner))]
    async fn set_param_inner(
        inner: Arc<RwLock<ComponentInner>>,
        parameter: Parameter,
    ) -> Result<Parameter> {
        let target_system;
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let this_system;
        let this_component;
        let sender;
        let encoding;

        {
            let inner_guard = inner.read().await;

            target_system = inner_guard.system_id;
            this_system = inner_guard.system_id;
            this_component = inner_guard.component_id;
            sender = inner_guard.get_sender().await;
            encoding = inner_guard.encoding;
        }

        let header = MavHeader {
            system_id: this_system,
            component_id: this_component,
            sequence: 0,
        };
        let message = MavMessage::PARAM_SET(mavlink::ardupilotmega::PARAM_SET_DATA {
            target_system,
            target_component,
            param_id: Parameter::param_name_to_id(&parameter.name),
            param_value: parameter.param_value(encoding)?,
            param_type: parameter.param_type(),
        });

        loop {
            if let Err(error) = sender.send(Message::ToBeSent((header, message.clone()))) {
                warn!("Failed requesting parameter: {error:?}");

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }

            let recv_parameter =
                match Self::get_param_inner(inner.clone(), &parameter.name, true).await {
                    Ok(parameter) => parameter,
                    Err(error) => {
                        warn!("Failed getting parameter: {error:?}");

                        continue;
                    }
                };

            let (Ok(sent_value), Ok(recv_value)) = (
                recv_parameter.param_value(encoding),
                parameter.param_value(encoding),
            ) else {
                warn!("Failed checking param!");

                continue;
            };

            if recv_value != sent_value {
                return Err(anyhow!(
                    "Failed setting parameter {:?}: Autopilot didn't accept the value: Sent {sent_value:?}, got {recv_value:?}",
                    parameter.name
                ));
            }

            return Ok(recv_parameter);
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn encoding(&self) -> ParamEncodingType {
        self.inner.read().await.encoding
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

struct ComponentInner {
    pub system_id: u8,
    pub component_id: u8,
    pub encoding: ParamEncodingType,
    pub parameters: HashMap<String, Parameter>,
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
            parameters: HashMap::with_capacity(2048),
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
