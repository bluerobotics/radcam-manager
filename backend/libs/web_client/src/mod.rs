use std::net::SocketAddr;

use anyhow::Result;
use reqwest::RequestBuilder;
use serde::{Serialize, de::DeserializeOwned};
use tracing::*;
use url::Url;
use url_params_serializer::to_url_params;

#[instrument(level = "debug", skip_all)]
pub async fn get<
    D: DeserializeOwned + std::fmt::Debug,
    B: Serialize + std::fmt::Debug,
    Q: Serialize + std::fmt::Debug,
>(
    address: &SocketAddr,
    path: &str,
    body_data: B,
    query_params: Q,
) -> Result<D> {
    let url = Url::parse_with_params(
        &format!("http://{address}/{path}"),
        to_url_params(query_params),
    )?;

    send_request(
        reqwest::Client::new().get(url),
        serde_json::to_string(&body_data)?,
    )
    .await
}

#[instrument(level = "debug", skip_all)]
pub async fn post<
    D: DeserializeOwned + std::fmt::Debug,
    B: Serialize + std::fmt::Debug,
    Q: Serialize + std::fmt::Debug,
>(
    address: &SocketAddr,
    path: &str,
    body_data: B,
    query_params: Q,
) -> Result<D> {
    let url = Url::parse_with_params(
        &format!("http://{address}/{path}"),
        to_url_params(query_params),
    )?;

    send_request(
        reqwest::Client::new().post(url),
        serde_json::to_string(&body_data)?,
    )
    .await
}

#[instrument(level = "debug", skip_all)]
pub async fn delete<
    D: DeserializeOwned + std::fmt::Debug,
    B: Serialize + std::fmt::Debug,
    Q: Serialize + std::fmt::Debug,
>(
    address: &SocketAddr,
    path: &str,
    body_data: B,
    query_params: Q,
) -> Result<D> {
    let url = Url::parse_with_params(
        &format!("http://{address}/{path}"),
        to_url_params(query_params),
    )?;

    send_request(
        reqwest::Client::new().delete(url),
        serde_json::to_string(&body_data)?,
    )
    .await
}

#[instrument(level = "debug", skip_all)]
pub async fn send_request<D: DeserializeOwned + std::fmt::Debug>(
    request_builder: RequestBuilder,
    body_data: String,
) -> Result<D> {
    let content = request_builder
        .timeout(std::time::Duration::from_secs(30))
        .body(body_data)
        .header("accept", "application/json")
        .header("Content-Type", "application/json")
        .send()
        .await
        .inspect_err(|error| {
            warn!("Error from send(): {error:#?}");
        })?
        .error_for_status()
        .inspect_err(|error| {
            warn!("Error from error_for_status(): {error:#?}");
        })?
        .text()
        .await
        .inspect_err(|error| {
            warn!("Error from text(): {error:#?}");
        })?;

    if std::any::type_name::<D>() == "()" {
        return Ok(serde_json::from_str("null")?);
    }

    let data = serde_json::from_str(&content).inspect_err(|error| {
        warn!("Error from serde_json::from_str(): {error:#?}");
    })?;

    Ok(data)
}
