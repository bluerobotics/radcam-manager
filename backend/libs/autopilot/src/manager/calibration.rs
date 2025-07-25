use anyhow::Result;
use tracing::*;
use uuid::Uuid;

use crate::{api, manager::Manager};

impl Manager {
    #[instrument(level = "debug", skip(self, _points))]

    pub async fn update_closest_points(
        &mut self,
        camera_uuid: &Uuid,
        _points: &api::FocusZoomPoints,
        overwrite: bool,
    ) -> Result<bool> {
        let should_update_script = false;

        warn!("Skipping: unimplemented");

        Ok(should_update_script)
    }

    #[instrument(level = "debug", skip(self, _points))]
    pub async fn update_furthest_points(
        &mut self,
        camera_uuid: &Uuid,
        _points: &api::FocusZoomPoints,
        overwrite: bool,
    ) -> Result<bool> {
        let should_update_script = false;

        warn!("Skipping: unimplemented");

        Ok(should_update_script)
    }
}
