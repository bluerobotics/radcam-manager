use clap::Parser;
use once_cell::sync::OnceCell;
use tokio::net::lookup_host;
use tracing::*;

static MANAGER: OnceCell<Manager> = OnceCell::new();

struct Manager {
    clap_matches: Args,
}

#[derive(Debug, Parser)]
#[command(
    version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"),
    author = option_env!("CARGO_PKG_AUTHORS").unwrap_or("?"),
    about = option_env!("CARGO_PKG_DESCRIPTION").unwrap_or("?"),
)]
pub struct Args {
    /// Turns all log categories up to Debug, for more information check RUST_LOG env variable.
    #[arg(short, long)]
    verbose: bool,

    /// Sets the IP and port that the server will be provided.
    #[arg(long, default_value = "0.0.0.0:8080")]
    web_server: String,

    /// Turns all log categories up to Trace to the log file, for more information check RUST_LOG env variable.
    #[arg(long)]
    enable_tracing_level_log_file: bool,

    /// Specifies the path in which the logs will be stored.
    #[arg(long, default_value = "./logs")]
    log_path: Option<String>,

    /// Sets the default version used by the REST API, this will remove the prefix used by its path.
    #[arg(long, default_value = "1", value_names = ["1"])]
    default_api_version: u8,

    /// Sets the Mavlink Camera Manager address.
    #[arg(long, default_value = "127.0.0.1:6020")]
    mcm_address: String,

    /// Sets the file path for the autopilot lua script to control zoom and focus
    #[arg(long, default_value = "./scripts/radcam.lua")]
    autopilot_scripts_file: Option<String>,
}

/// Constructs our manager, Should be done inside main
#[instrument(level = "debug")]
pub fn init() {
    init_with(Args::parse());
}

/// Constructs our manager, Should be done inside main
#[instrument(level = "debug")]
pub fn init_with(args: Args) {
    MANAGER.get_or_init(|| Manager { clap_matches: args });
}

/// Local acessor to the parsed Args
fn args() -> &'static Args {
    &MANAGER.get().unwrap().clap_matches
}

/// Checks if the verbosity parameter was used
#[instrument(level = "debug")]
pub fn is_verbose() -> bool {
    args().verbose
}

#[instrument(level = "debug")]
pub fn is_tracing() -> bool {
    args().enable_tracing_level_log_file
}

#[instrument(level = "debug")]
pub fn log_path() -> String {
    let log_path = args()
        .log_path
        .clone()
        .expect("Clap arg \"log-path\" should always be \"Some(_)\" because of the default value.");

    shellexpand::full(&log_path)
        .expect("Failed to expand path")
        .to_string()
}

#[instrument(level = "debug")]
pub fn command_line_string() -> String {
    std::env::args().collect::<Vec<String>>().join(" ")
}

/// Returns a pretty string of the current Args struct
#[instrument(level = "debug")]
pub fn command_line() -> String {
    format!("{:#?}", args())
}

#[instrument(level = "debug")]
pub async fn web_server() -> std::net::SocketAddr {
    let address = &args().web_server;

    resolve_address(address).await.unwrap()
}

#[instrument(level = "debug")]
pub async fn mcm_address() -> std::net::SocketAddr {
    let address = &args().mcm_address;

    resolve_address(address).await.unwrap()
}

#[instrument(level = "debug")]
pub fn autopilot_scripts_file() -> String {
    let autopilot_scripts_file = args()
        .autopilot_scripts_file
        .clone()
        .expect("Clap arg \"autopilot-scripts-file\" should always be \"Some(_)\" because of the default value.");

    if !autopilot_scripts_file.ends_with(".lua") {
        panic!("Clap arg \"autopilot-scripts-file\" Should always end with \".lua\"");
    }

    shellexpand::full(&autopilot_scripts_file)
        .expect("Failed to expand path")
        .to_string()
}

#[instrument(level = "debug")]
pub fn default_api_version() -> u8 {
    args().default_api_version
}

async fn resolve_address(address: &str) -> std::io::Result<std::net::SocketAddr> {
    let mut addrs = lookup_host(address).await?;
    addrs.next().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::AddrNotAvailable,
            "Unable to resolve address",
        )
    })
}
