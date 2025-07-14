use anyhow::Result;
use tracing::*;

use autopilot;
use radcam_manager::{logger, web};

pub mod cli;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    cli::init();

    logger::init(cli::log_path(), cli::is_verbose(), cli::is_tracing());

    info!(
        "{}, version: {}-{}, build date: {}",
        option_env!("CARGO_PKG_NAME").unwrap_or("?"),
        option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"),
        option_env!("VERGEN_GIT_SHA").unwrap_or("?"),
        option_env!("VERGEN_BUILD_DATE").unwrap_or("?"),
    );
    info!(
        "Starting at {}",
        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
    );
    debug!("Command line call: {}", cli::command_line_string());
    debug!("Command line input struct call: {}", cli::command_line());

    mcm_client::init(cli::mcm_address().await).await;

    autopilot::init(
        cli::settings_file(),
        None,
        cli::mavlink_connection_string(),
        cli::mavlink_system_id(),
        cli::mavlink_component_id(),
    )
    .await
    .unwrap();

    web::run(cli::web_server().await, cli::default_api_version()).await;

    Ok(())
}
