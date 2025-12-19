use anyhow::Result;
use std::collections::HashSet;
use tracing::*;
use uuid::Uuid;

#[instrument(level = "debug")]
pub async fn camera_initialization_task() -> Result<()> {
    let mut initialized_cameras: HashSet<Uuid> = HashSet::new();

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let cameras = mcm_client::cameras().await;

        for (camera_uuid, _camera) in cameras {
            if initialized_cameras.contains(&camera_uuid) {
                continue;
            }

            if let Err(error) = radcam_commands::apply_recommended_settings(camera_uuid).await {
                error!("Failed to apply settings to {camera_uuid:?}: {error:?}");
            }

            initialized_cameras.insert(camera_uuid);
        }
    }
}
