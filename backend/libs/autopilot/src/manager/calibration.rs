use anyhow::Result;
use tracing::*;
use uuid::Uuid;

use crate::{api, manager::Manager};

impl Manager {
    #[instrument(level = "debug", skip(_points))]
    pub async fn update_closest_points(
        _camera_uuid: &Uuid,
        _points: &api::FocusZoomPoints,
        _overwrite: bool,
    ) -> Result<bool> {
        warn!("Skipping: unimplemented");
        Ok(false)
    }

    #[instrument(level = "debug", skip(_points))]
    pub async fn update_furthest_points(
        _camera_uuid: &Uuid,
        _points: &api::FocusZoomPoints,
        _overwrite: bool,
    ) -> Result<bool> {
        warn!("Skipping: unimplemented");
        Ok(false)
    }
}
