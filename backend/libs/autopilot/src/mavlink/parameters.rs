use std::sync::Arc;

use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use mavlink::{
    MavHeader,
    ardupilotmega::{MavMessage, MavProtocolCapability},
};
use tokio::sync::broadcast;
use tracing::*;

use crate::{
    mavlink::{ComponentInner, MavlinkComponent, connection::Message},
    parameters::Parameter,
};

#[derive(Debug, Default, Clone, Copy)]
pub enum ParamEncodingType {
    CCast,
    ByteWise,
    #[default]
    Unsupported,
}

impl MavlinkComponent {
    #[instrument(level = "debug", skip(inner))]
    pub(crate) async fn configure_parameter_encoding(inner: Arc<ComponentInner>) {
        let target_system;
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let this_system;
        let this_component;
        let sender;
        let mut receiver;

        {
            target_system = inner.system_id;
            this_system = inner.system_id;
            this_component = inner.component_id;
            sender = inner.get_sender().await;
            receiver = inner.get_receiver().await;
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
        *inner.encoding.write().await = encoding;
    }

    #[instrument(level = "debug", skip(inner))]
    pub(crate) async fn update_all_params(inner: Arc<ComponentInner>) {
        let target_system = inner.system_id;
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let this_system = inner.system_id;
        let this_component = inner.component_id;
        let sender = inner.get_sender().await;
        let mut receiver = inner.get_receiver().await;
        let encoding = *inner.encoding.read().await;

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
        let mut parameters = IndexMap::with_capacity(2048);

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

                trace!(
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

        *inner.parameters.write().await = parameters;
    }

    #[instrument(level = "debug", skip(inner))]
    pub(crate) async fn params_sync_task(inner: Arc<ComponentInner>) {
        let mut receiver = inner.get_receiver().await;
        let encoding = *inner.encoding.read().await;

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
                .parameters
                .write()
                .await
                .entry(parameter.name.clone())
                .and_modify(|v| {
                    if v.value != parameter.value && v.name != "STAT_RUNTIME" {
                        debug!(
                            "Parameter {:?} updated from {:?} to {:?}",
                            v.name, v.value, parameter.value,
                        );
                    }
                    *v = parameter.clone()
                })
                .or_insert_with(|| {
                    trace!("New parameter added: {parameter:?}");
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
        inner: Arc<ComponentInner>,
        param_name: &str,
        skip_cache: bool,
    ) -> Result<Parameter> {
        if !skip_cache && let Some(parameter) = inner.parameters.read().await.get(param_name) {
            trace!("Got parameter from cache!");
            return Ok(parameter.clone());
        }

        let target_system = inner.system_id;
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let this_system = inner.system_id;
        let this_component = inner.component_id;
        let sender = inner.get_sender().await;
        let mut receiver = inner.get_receiver().await;
        let encoding = *inner.encoding.read().await;

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
                param_id: param_name.into(),
            });

        let mut max_retries = 5;
        while max_retries > 0 {
            max_retries -= 1;

            debug!("Sent param request {:?}", param_name);
            if let Err(error) = sender.send(Message::ToBeSent((header, message.clone()))) {
                warn!("Failed requesting parameter {param_name:?}: {error:?}");

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }

            let wait_for_param = Self::wait_for_param_inner(&mut receiver, param_name, encoding);

            match tokio::time::timeout(tokio::time::Duration::from_secs(5), wait_for_param).await {
                Ok(res @ Ok(_)) => return res,
                Ok(Err(error)) => {
                    warn!("Retrying after error: {error:?}");
                }
                Err(_) => {
                    warn!("Timeout waiting for param {:?}, retrying", param_name);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }

        Err(anyhow!("Failed after too many tries"))
    }

    #[instrument(level = "debug", skip(inner))]
    async fn wait_for_param(inner: Arc<ComponentInner>, param_name: &str) -> Result<Parameter> {
        let mut receiver = inner.get_receiver().await;
        let encoding = *inner.encoding.read().await;

        let mut max_retries = 5;
        while max_retries > 0 {
            max_retries -= 1;
            let wait_for_param = Self::wait_for_param_inner(&mut receiver, param_name, encoding);

            match tokio::time::timeout(tokio::time::Duration::from_secs(5), wait_for_param).await {
                Ok(res @ Ok(_)) => return res,
                Ok(Err(error)) => {
                    warn!("Retrying after error: {error:?}");
                }
                Err(_) => {
                    warn!("Timeout waiting for param {:?}, retrying", param_name);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }

        Err(anyhow!("Failed after too many tries"))
    }

    #[instrument(level = "trace", skip(receiver))]
    async fn wait_for_param_inner(
        receiver: &mut broadcast::Receiver<Message>,
        param_name: &str,
        encoding: ParamEncodingType,
    ) -> Result<Parameter> {
        loop {
            let (_header, message) = match receiver.recv().await {
                Ok(Message::Received(inner)) => inner,
                Ok(Message::ToBeSent(_)) => continue,
                Err(error) => return Err(anyhow::anyhow!("Receiver error: {error:?}")),
            };

            let MavMessage::PARAM_VALUE(data) = message else {
                continue;
            };

            let parameter = Parameter::try_new(&data, encoding).map_err(|error| {
                anyhow::anyhow!("Failed creating parameter from {data:?}: {error:?}")
            })?;

            if parameter.name != param_name {
                continue;
            }

            debug!(
                "Received param [{}/{}] {parameter:?}...",
                data.param_index.saturating_add(1),
                data.param_count
            );

            return Ok(parameter);
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn set_param(&self, parameter: Parameter) -> Result<Parameter> {
        Self::set_param_inner(self.inner.clone(), parameter).await
    }

    #[instrument(level = "debug", skip(inner))]
    async fn set_param_inner(
        inner: Arc<ComponentInner>,
        parameter: Parameter,
    ) -> Result<Parameter> {
        let target_system = inner.system_id;
        let target_component = mavlink::ardupilotmega::MavComponent::MAV_COMP_ID_AUTOPILOT1 as u8;
        let this_system = inner.system_id;
        let this_component = inner.component_id;
        let sender = inner.get_sender().await;
        let encoding = *inner.encoding.read().await;

        let header = MavHeader {
            system_id: this_system,
            component_id: this_component,
            sequence: 0,
        };
        let message = MavMessage::PARAM_SET(mavlink::ardupilotmega::PARAM_SET_DATA {
            target_system,
            target_component,
            param_id: parameter.name.as_str().into(),
            param_value: parameter.param_value(encoding)?,
            param_type: parameter.param_type(),
        });

        loop {
            if let Err(error) = sender.send(Message::ToBeSent((header, message.clone()))) {
                warn!("Failed requesting parameter: {error:?}");

                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                continue;
            }

            let recv_parameter = match Self::wait_for_param(inner.clone(), &parameter.name).await {
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
        *self.inner.encoding.read().await
    }
}
