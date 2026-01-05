use std::net::SocketAddr;

use anyhow::{Context, Result, anyhow};
use indexmap::IndexMap;
use tracing::*;

use crate::mcm_types::{
    ApiVideoSource, AuthenticateOnvifDeviceRequest, CaptureConfiguration, Format, Info,
    OnvifDevice, OnvifDeviceInformation, PostStream, RemoveStream, StreamInformation, StreamStatus,
    UnauthenticateOnvifDeviceRequest, VideoCaptureConfiguration, VideoEncodeType, VideoSourceOnvif,
    VideoSourceOnvifType, VideoSourceType,
};

use super::{Camera, Credentials, Stream};

pub struct MCMClient {
    pub address: SocketAddr,
    _info: Info,
}

impl MCMClient {
    #[instrument(level = "debug")]
    pub async fn try_new(address: &SocketAddr) -> Result<Self> {
        let _info = Self::get_info(address).await?;

        let version = semver::Version::parse(&_info.version)?;
        let supported = semver::VersionReq::parse("0.2.4")?;

        if !supported.matches(&version) {
            return Err(anyhow!(
                "Version {version:?} is not supported. Supported: {supported:?}"
            ));
        }

        Ok(Self {
            address: *address,
            _info,
        })
    }

    #[instrument(level = "debug")]
    async fn get_info(address: &SocketAddr) -> Result<Info> {
        web_client::get(address, "info", (), ()).await
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_onvif_devices(&self) -> Result<Vec<OnvifDevice>> {
        web_client::get(&self.address, "onvif/devices", (), ()).await
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn get_radcams(&self) -> Result<Vec<Camera>> {
        let devices = self.get_onvif_devices().await?;

        let radcam_devices = radcams_from_onvif_devices(devices);

        Ok(radcam_devices)
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_video_sources(&self) -> Result<Vec<ApiVideoSource>> {
        web_client::get(&self.address, "v4l", (), ()).await
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn get_radcam_video_sources(&self) -> Result<Vec<ApiVideoSource>> {
        let sources = self
            .get_video_sources()
            .await?
            .into_iter()
            .filter(|source| {
                // Firmware <= v11
                source.name.eq("HDIPC - IPCamera (IPCamera)")
                // Firmware >= v13
                    || source.name.eq("UnderwaterCam - IPCamera (UnderwaterCam)")
            })
            .collect();

        Ok(sources)
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn get_radcam_streams(&self) -> Result<Vec<Stream>> {
        let radcam_streams = self
            .get_streams()
            .await?
            .into_iter()
            .filter_map(|device| {
                let VideoSourceType::Onvif(VideoSourceOnvif {
                    name,
                    source: VideoSourceOnvifType::Onvif(endpoint),
                    device_information:
                        OnvifDeviceInformation {
                            manufacturer,
                            model,
                            ..
                        },
                }) = &device.video_and_stream.video_source
                else {
                    return None;
                };

                // Firmware <= v11
                let is_old_firmware = name.eq("HDIPC - IPCamera (IPCamera)")
                    && model.eq("HDIPC")
                    && manufacturer.eq("IPCamera");
                // Firmware >= v13
                let is_new_firmware = name.eq("UnderwaterCam - IPCamera (UnderwaterCam)")
                    && model.eq("UnderwaterCam")
                    && manufacturer.eq("IPCamera");

                if !(is_old_firmware || is_new_firmware) {
                    return None;
                }

                let Ok(source_endpoint) = endpoint.parse() else {
                    return None;
                };

                let stream_endpoints = device.video_and_stream.stream_information.endpoints;

                Some(Stream {
                    name: name.to_owned(),
                    source_endpoint,
                    stream_endpoints,
                })
            })
            .collect::<Vec<Stream>>();

        // let endpoints = self
        //     .get_streams()
        //     .await?
        //     .into_iter()
        //     .flat_map(|stream| stream.video_and_stream.stream_information.endpoints)
        //     .collect::<Vec<Url>>();

        Ok(radcam_streams)
    }

    #[instrument(level = "debug", skip(self))]
    pub(crate) async fn authenticate(&self, camera: &Camera) -> Result<()> {
        let credentials = camera.credentials.clone().context("No credentials set")?;

        let data = AuthenticateOnvifDeviceRequest {
            device_uuid: camera.uuid,
            username: credentials.username,
            password: credentials.password,
        };

        web_client::post(&self.address, "onvif/authentication", (), data).await
    }

    #[instrument(level = "debug", skip(self))]
    pub(crate) async fn unauthenticate(&self, camera: &Camera) -> Result<()> {
        let data = UnauthenticateOnvifDeviceRequest {
            device_uuid: camera.uuid,
        };

        web_client::delete(&self.address, "onvif/authentication", (), data).await
    }

    #[instrument(level = "debug", skip(self))]
    async fn get_streams(&self) -> Result<Vec<StreamStatus>> {
        web_client::get(&self.address, "streams", (), ()).await
    }

    #[instrument(level = "debug", skip(self))]
    pub async fn create_stream(&self, source: ApiVideoSource) -> Result<Vec<StreamStatus>> {
        let formats = source
            .formats
            .into_iter()
            .filter(|format| matches!(format.encode, VideoEncodeType::H264 | VideoEncodeType::H265))
            .collect::<Vec<Format>>();

        // TODO: Decide a way to get the format
        let format = formats.first().context("Empty formats")?;
        let encode = format.encode.to_owned();
        let size = format.sizes.first().context("Empty sizes")?.to_owned();
        let frame_interval = size
            .intervals
            .first()
            .context("Empty intervals")?
            .to_owned();

        // TODO: When using DHCP, there's no guarantee that the IP of the camera is kept the same, so we NEED to find a more robust ID
        let id = {
            let source: url::Url = source.source.clone().parse()?;

            let host = source.host_str().context("No host")?;
            let stream_id = source.path().chars().last().context("No path")?;

            format!("{host}/{stream_id}")
        };

        let data = PostStream {
            name: format!("RadCam {id}"),
            source: source.source,
            stream_information: StreamInformation {
                endpoints: vec![format!("rtsp://0.0.0.0:8554/radcam_{id}").parse()?],
                configuration: CaptureConfiguration::Video(VideoCaptureConfiguration {
                    encode,
                    height: size.height,
                    width: size.width,
                    frame_interval,
                }),
                extended_configuration: None,
            },
        };

        web_client::post(&self.address, "streams", data, ()).await
    }

    #[instrument(level = "debug", skip(self))]
    async fn delete_stream(&self) -> Result<Vec<Camera>> {
        let data = RemoveStream { name: todo!() };

        let devices = web_client::delete(&self.address, "delete_stream", (), data).await?;

        Ok(radcams_from_onvif_devices(devices))
    }
}

fn radcams_from_onvif_devices(devices: Vec<OnvifDevice>) -> Vec<Camera> {
    devices
        .iter()
        .filter_map(|device| {
            if device.name != Some("hd".to_string())
                || device.hardware != Some("HW0100302".to_string())
            {
                trace!("Skipping unknown {device:?}");

                return None;
            };

            trace!("RadCam found: {device:?}");

            Some(Camera {
                hostname: device.ip,
                uuid: device.uuid,
                credentials: Some(Credentials {
                    username: "admin".to_string(),
                    password: "blue".to_string(),
                }),
                streams: IndexMap::new(),
            })
        })
        .collect()
}
