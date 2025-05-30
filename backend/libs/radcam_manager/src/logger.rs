use std::{
    io::{self, Write},
    sync::Mutex,
};

use once_cell::sync::OnceCell;
use ringbuffer::{AllocRingBuffer, RingBuffer};
use tokio::sync::broadcast::{Receiver, Sender};
use tracing::metadata::LevelFilter;
use tracing_log::LogTracer;
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, MakeWriter},
    layer::SubscriberExt,
};

static MANAGER: OnceCell<Manager> = OnceCell::new();
pub static HISTORY: OnceCell<Mutex<History>> = OnceCell::new();

#[derive(Debug, Default)]
pub struct Manager {
    pub process: Option<tokio::task::JoinHandle<()>>,
}

pub struct History {
    pub history: AllocRingBuffer<String>,
    pub sender: Sender<String>,
}

struct BroadcastWriter {
    sender: Sender<String>,
}

impl Write for BroadcastWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let message = String::from_utf8_lossy(buf).to_string();
        let _ = self.sender.send(message);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BroadcastMakeWriter {
    sender: Sender<String>,
}

impl<'a> MakeWriter<'a> for BroadcastMakeWriter {
    type Writer = BroadcastWriter;

    fn make_writer(&'a self) -> Self::Writer {
        BroadcastWriter {
            sender: self.sender.clone(),
        }
    }
}

impl Default for History {
    fn default() -> Self {
        let (sender, _receiver) = tokio::sync::broadcast::channel(100);
        Self {
            history: AllocRingBuffer::new(10 * 1024),
            sender,
        }
    }
}

impl History {
    pub fn push(&mut self, message: String) {
        self.history.push(message.clone());
        let _ = self.sender.send(message);
    }

    pub fn subscribe(&self) -> (Receiver<String>, Vec<String>) {
        let reader = self.sender.subscribe();
        (reader, self.history.to_vec())
    }
}

// Start logger, should be done inside main
pub fn init(log_path: String, is_verbose: bool, is_tracing: bool) {
    // Redirect all logs from libs using "Log"
    LogTracer::init_with_filter(tracing::log::LevelFilter::Trace).expect("Failed to set logger");

    // Configure the console log
    let console_env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            if is_verbose {
                EnvFilter::new(LevelFilter::DEBUG.to_string())
            } else {
                EnvFilter::new(LevelFilter::INFO.to_string())
            }
        })
        // Hyper is used for http request by our thread leak test
        // And it's pretty verbose when it's on
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());
    let console_layer = fmt::Layer::new()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(fmt::format::FmtSpan::NONE)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_filter(console_env_filter);

    // Configure the file log
    let file_env_filter = if is_tracing {
        EnvFilter::new(LevelFilter::TRACE.to_string())
    } else {
        EnvFilter::new(LevelFilter::DEBUG.to_string())
    };
    let file_appender = custom_rolling_appender(
        log_path,
        tracing_appender::rolling::Rotation::HOURLY,
        "radcam-manager",
        "log",
    );
    let file_layer = fmt::Layer::new()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(fmt::format::FmtSpan::NONE)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_filter(file_env_filter);

    // Configure the server log
    let server_env_filter = if is_tracing {
        EnvFilter::new(LevelFilter::TRACE.to_string())
    } else {
        EnvFilter::new(LevelFilter::DEBUG.to_string())
    };
    let (tx, mut rx) = tokio::sync::broadcast::channel(100);
    let server_layer = fmt::Layer::new()
        .with_writer(BroadcastMakeWriter { sender: tx.clone() })
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(fmt::format::FmtSpan::NONE)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_filter(server_env_filter);

    let history = HISTORY.get_or_init(|| Mutex::new(History::default()));

    MANAGER.get_or_init(|| Manager {
        process: Some(tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(message) => {
                        history.lock().unwrap().push(message);
                    }
                    Err(_error) => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }
            }
        })),
    });

    // Configure the default subscriber
    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .with(server_layer);
    tracing::subscriber::set_global_default(subscriber).expect("Unable to set a global subscriber");
}

fn custom_rolling_appender<P: AsRef<std::path::Path>>(
    dir: P,
    rotation: tracing_appender::rolling::Rotation,
    prefix: &str,
    suffix: &str,
) -> tracing_appender::rolling::RollingFileAppender {
    tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(rotation)
        .filename_prefix(prefix)
        .filename_suffix(suffix)
        .build(dir)
        .expect("failed to initialize rolling file appender")
}
