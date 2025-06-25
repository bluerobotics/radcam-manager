use anyhow::{Context, Result};
use chrono::Utc;
use std::{fs, path::Path};
use tracing::*;

use crate::ZoomAndFocusConfig;

pub fn read_settings(settings_file: &str) -> Result<ZoomAndFocusConfig> {
    let path = Path::new(settings_file);

    if path.exists() {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read settings file: {settings_file:?}"))?;
        let config: ZoomAndFocusConfig = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse JSON from: {settings_file:?}"))?;

        debug!("Loaded settings from {settings_file:?}");

        return Ok(config);
    }

    // Try to find the latest backup file if the main file doesn't exist
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let backups = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("settings.json.")
                && entry
                    .path()
                    .extension()
                    .map(|e| e == "bak")
                    .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    if let Some(latest_backup) = backups
        .iter()
        .max_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok())
    {
        let contents = fs::read_to_string(latest_backup.path())
            .with_context(|| format!("Failed to read backup file: {:?}", latest_backup.path()))?;
        let config: ZoomAndFocusConfig = serde_json::from_str(&contents).with_context(|| {
            format!(
                "Failed to parse JSON from backup: {:?}",
                latest_backup.path()
            )
        })?;

        warn!(
            "Loaded settings from backup file: {:?}",
            latest_backup.path()
        );

        return Ok(config);
    }

    warn!("No settings file or backup found. Using default configuration.");
    Ok(ZoomAndFocusConfig::default())
}

pub fn write_settings(settings_file: &str, config: &ZoomAndFocusConfig) -> Result<()> {
    let path = Path::new(settings_file);

    // Serialize the new config to JSON now
    let new_contents =
        serde_json::to_string_pretty(config).context("Failed to serialize config to JSON")?;

    // Check if file exists and contents differ
    if path.exists() {
        let current_contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read existing settings file: {settings_file:?}"))?;

        if current_contents == new_contents {
            trace!("Settings unchanged, not writing or backing up");
            return Ok(());
        }

        let now = Utc::now().timestamp();
        let backup_path = path.with_file_name(format!("settings.json.{now}.bak"));

        fs::copy(path, &backup_path)
            .with_context(|| format!("Failed to back up file to {backup_path:?}"))?;
        debug!("Created backup: {backup_path:?}");
    }

    fs::write(path, &new_contents)
        .with_context(|| format!("Failed to write settings file to {settings_file:?}"))?;

    debug!("Wrote new settings to {settings_file:?}:\n{config:#?}");

    Ok(())
}
