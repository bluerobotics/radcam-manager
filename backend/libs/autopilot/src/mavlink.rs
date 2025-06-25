use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use mavlink::{
    self, AsyncMavConnection, MavHeader, Message, MessageData,
    ardupilotmega::{MavMessage, MavProtocolCapability},
    error::MessageReadError,
};
use tokio::sync::RwLock;
use tracing::*;

use crate::parameters::Parameter;

#[derive(Debug)]
pub struct MavlinkComponent {
    inner: Arc<RwLock<ComponentInner>>,
    heartbeat_handle: tokio::task::JoinHandle<()>,
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

        let heartbeat_handle = tokio::task::spawn(Self::heartbeat_task(inner.clone()));

        Self::configure_parameter_encoding(inner.clone()).await;

        Self::update_all_params(inner.clone()).await;

        Self {
            inner,
            heartbeat_handle,
        }
    }

    pub async fn component_id(&self) -> u8 {
        self.inner.read().await.component_id
    }

    pub async fn system_id(&self) -> u8 {
        self.inner.read().await.system_id
    }

    pub async fn send(&self, sequence: Option<u8>, data: &MavMessage) {
        let mut inner_guard = self.inner.write().await;

        inner_guard.send(sequence, data).await;
    }

    pub async fn recv(
        &self,
        system_id: u8,
        component_id: u8,
        message_id: u32,
    ) -> (MavHeader, MavMessage) {
        let mut inner_guard = self.inner.write().await;

        inner_guard.recv(system_id, component_id, message_id).await
    }

    #[instrument(level = "debug", skip(inner))]
    async fn heartbeat_task(inner: Arc<RwLock<ComponentInner>>) {
        let mut sequence = 1;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let mut inner_guard = inner.write().await;

            let data = MavMessage::HEARTBEAT(mavlink::ardupilotmega::HEARTBEAT_DATA {
                custom_mode: 0,
                mavtype: mavlink::ardupilotmega::MavType::MAV_TYPE_CAMERA,
                autopilot: mavlink::ardupilotmega::MavAutopilot::MAV_AUTOPILOT_INVALID,
                base_mode: mavlink::ardupilotmega::MavModeFlag::empty(),
                system_status: mavlink::ardupilotmega::MavState::MAV_STATE_STANDBY,
                mavlink_version: 0x3,
            });

            inner_guard.send(Some(sequence), &data).await;

            sequence = sequence.wrapping_add(1);
        }
    }

    #[instrument(level = "debug", skip(inner))]
    async fn configure_parameter_encoding(inner: Arc<RwLock<ComponentInner>>) {
        let mut inner_guard = inner.write().await;
        let system_id = inner_guard.system_id;
        let component_id = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;

        let data = MavMessage::AUTOPILOT_VERSION_REQUEST(
            mavlink::ardupilotmega::AUTOPILOT_VERSION_REQUEST_DATA {
                target_system: system_id,
                target_component: component_id,
            },
        );

        debug!("Getting parameter encoding from target {system_id}:{component_id}...");

        let encoding = loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            debug!("Requesting Autopilot Version...");

            inner_guard.send(None, &data).await;

            let (_header, message) = inner_guard
                .recv(
                    system_id,
                    component_id,
                    mavlink::ardupilotmega::AUTOPILOT_VERSION_DATA::ID,
                )
                .await;

            let MavMessage::AUTOPILOT_VERSION(data) = message else {
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
        inner_guard.encoding = encoding;
    }

    #[instrument(level = "debug", skip(inner))]
    async fn update_all_params(inner: Arc<RwLock<ComponentInner>>) {
        let mut inner_guard = inner.write().await;
        let system_id = inner_guard.system_id;
        let component_id = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;

        let data =
            MavMessage::PARAM_REQUEST_LIST(mavlink::ardupilotmega::PARAM_REQUEST_LIST_DATA {
                target_system: system_id,
                target_component: component_id,
            });

        debug!("Getting parameter list from target {system_id}:{component_id}...");

        let mut current_param = 0;
        let mut params_to_refetch = Vec::with_capacity(2048);

        'send: loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            debug!("Requesting parameter list...");

            inner_guard.send(None, &data).await;

            loop {
                let (_header, message) = inner_guard
                    .recv(
                        system_id,
                        component_id,
                        mavlink::ardupilotmega::PARAM_VALUE_DATA::ID,
                    )
                    .await;

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

                let parameter = Parameter::new(&data, inner_guard.encoding);

                debug!(
                    "Received param [{}/{}] {parameter:?}...",
                    data.param_index + 1,
                    data.param_count
                );

                inner_guard
                    .parameters
                    .entry(parameter.name.clone())
                    .and_modify(|v| *v = parameter.clone())
                    .or_insert(parameter);

                if (data.param_index + 1) == data.param_count {
                    if inner_guard.parameters.len() == data.param_count as usize {
                        debug!("Received all {:?} parameters", inner_guard.parameters.len());
                        break 'send;
                    }

                    debug!(
                        "Received {:?} parameters, but missed {:?}: {:?}. Retrying...",
                        inner_guard.parameters.len(),
                        params_to_refetch.len(),
                        params_to_refetch
                    );
                    continue 'send;
                }
            }
        }
    }

    #[instrument(level = "debug", skip(inner))]
    async fn params_sync_task(inner: Arc<RwLock<ComponentInner>>) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let mut inner_guard = inner.write().await;

            let system_id = inner_guard.system_id;
            let component_id = inner_guard.component_id;

            let (_header, message) = inner_guard
                .recv(
                    system_id,
                    component_id,
                    mavlink::ardupilotmega::PARAM_VALUE_DATA::ID,
                )
                .await;

            let MavMessage::PARAM_VALUE(data) = message else {
                continue;
            };

            let parameter = Parameter::new(&data, inner_guard.encoding);

            inner_guard
                .parameters
                .entry(parameter.name.clone())
                .and_modify(|v| *v = parameter);
        }
    }

    async fn get_param(inner: Arc<RwLock<ComponentInner>>, param_name: &str) -> Result<Parameter> {
        let mut inner_guard = inner.write().await;

        let target_system = inner_guard.system_id;
        let target_component = mavlink::ardupilotmega::PARAM_VALUE_DATA::ID as u8;
        let this_system = inner_guard.system_id;
        let this_component = inner_guard.component_id;

        if !inner_guard.parameters.contains_key(param_name) {
            loop {
                inner_guard
                    .send(
                        None,
                        &MavMessage::PARAM_REQUEST_READ(
                            mavlink::ardupilotmega::PARAM_REQUEST_READ_DATA {
                                param_index: -1,
                                target_system,
                                target_component,
                                param_id: Parameter::param_name_to_id(param_name),
                            },
                        ),
                    )
                    .await;

                let (_header, message) = inner_guard
                    .recv(
                        this_system,
                        this_component,
                        mavlink::ardupilotmega::PARAM_VALUE_DATA::ID,
                    )
                    .await;

                let MavMessage::PARAM_VALUE(data) = message else {
                    continue;
                };

                let parameter = Parameter::new(&data, inner_guard.encoding);

                inner_guard
                    .parameters
                    .insert(param_name.to_string(), parameter);

                break;
            }
        }

        inner_guard
            .parameters
            .get(param_name)
            .context("Not found")
            .cloned()
    }

    async fn set_param(
        inner: Arc<RwLock<ComponentInner>>,
        parameter: Parameter,
    ) -> Result<Parameter> {
        todo!()
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn encoding(&self) -> ParamEncodingType {
        self.inner.read().await.encoding
    }
}

impl Drop for MavlinkComponent {
    fn drop(&mut self) {
        self.heartbeat_handle.abort();
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

struct Connection {
    address: String,
    inner: Option<Box<dyn AsyncMavConnection<MavMessage> + Sync + Send>>,
}

impl Connection {
    #[instrument(level = "debug")]
    async fn new(address: String) -> Self {
        let inner = Some(Self::connect(&address).await);

        Self { address, inner }
    }

    #[instrument(level = "debug")]
    async fn connect(address: &str) -> Box<dyn AsyncMavConnection<MavMessage> + Sync + Send> {
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

    pub async fn reconnect(&mut self) {
        self.inner.replace(Connection::connect(&self.address).await);
    }
}

impl ComponentInner {
    #[instrument(level = "debug")]
    pub async fn new(address: String, system_id: u8, component_id: u8) -> Self {
        let connection = Connection::new(address).await;

        Self {
            system_id,
            component_id,
            connection,
            parameters: HashMap::with_capacity(2048),
            encoding: ParamEncodingType::default(),
        }
    }

    async fn send(&mut self, sequence: Option<u8>, data: &MavMessage) {
        let header = self.header(sequence);

        loop {
            let Some(connection) = &self.connection.inner else {
                self.connection.reconnect().await;
                continue;
            };

            if let Err(error) = connection.send(&header, &data).await {
                error!("Failed sending mavlink message: {error:?}");

                self.connection.reconnect().await;
                continue;
            }

            break;
        }
    }

    async fn recv(
        &mut self,
        system_id: u8,
        component_id: u8,
        message_id: u32,
    ) -> (MavHeader, MavMessage) {
        loop {
            let Some(connection) = &self.connection.inner else {
                self.connection.reconnect().await;
                continue;
            };

            let (header, message) = match connection.recv().await {
                Ok(inner) => inner,
                Err(MessageReadError::Io(_)) => {
                    self.connection.reconnect().await;
                    continue;
                }
                Err(MessageReadError::Parse(_)) => continue,
            };

            if header.system_id != system_id
                || header.component_id != component_id
                || message.message_id() != message_id
            {
                continue;
            }

            return (header, message);
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub fn header(&self, sequence: Option<u8>) -> MavHeader {
        MavHeader {
            system_id: self.system_id,
            component_id: self.component_id,
            sequence: sequence.unwrap_or(1),
        }
    }
}
