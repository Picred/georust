//! Georust structured async logger.
//!
//! Design: a single background task owns the log file. Every `info!`/`warn!`/etc.
//! call just pushes a `LogLine` onto an unbounded mpsc channel 
//! The writer task is the *only* thing that ever touches the file

use std::path::Path;
use std::sync::OnceLock;
use std::sync::Mutex;
use std::time::Duration;

use sysinfo::{ProcessesToUpdate, System};
use serde::Serialize;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

pub use chrono;

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub time: String,
    pub level: LogLevel,
    pub module: LogModule,
    pub event: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogModule {
    Main,
    Authenticator,
    ConnectionManager,
    Database,
    JourneyTracking,
    Statistics,
    UserStateHandler,
}

impl std::fmt::Display for LogModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LogModule::Main => "main",
            LogModule::Authenticator => "authenticator",
            LogModule::ConnectionManager => "connection_manager",
            LogModule::Database => "database",
            LogModule::JourneyTracking => "journey_tracking",
            LogModule::Statistics => "statistics",
            LogModule::UserStateHandler => "user_state_handler",
        };
        write!(f, "{s}")
    }
}

/// Internal channel payload. `Flush` lets a caller (e.g. on shutdown) wait
/// until every line queued *before* it has actually hit disk, without giving
/// the writer task's file handle to anyone else.
enum LogCommand {
    Write(LogLine),
    Flush(oneshot::Sender<()>),
}

static SENDER: OnceLock<mpsc::UnboundedSender<LogCommand>> = OnceLock::new();

// Holds every background task `init()` spawns so they don't get silently 
// detached
static TASK_HANDLES: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

static MIN_LEVEL: OnceLock<LogLevel> = OnceLock::new();

#[doc(hidden)]
pub fn is_enabled(level: LogLevel) -> bool {
    level >= MIN_LEVEL.get().copied().unwrap_or(LogLevel::Debug)
}

#[derive(Debug)]
pub enum InitError {
    AlreadyInitialized,
    Io(std::io::Error),
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::AlreadyInitialized => write!(f, "Logger::init() called more than once"),
            InitError::Io(e) => write!(f, "failed to open log file: {e}"),
        }
    }
}
impl std::error::Error for InitError {}

pub struct Logger;

impl Logger {
    /// Opens `path` for appending and spawns the single writer task that will
    /// own the file handle for the lifetime of the process. Must be called
    /// once, before any `info!`/`warn!`/`debug!`/`error!` call
    ///
    /// `min_level` sets the level filter: any call at a lower severity than
    /// this is skipped
    ///
    /// `perf_interval` is how often a `LogModule::Statistics` snapshot of
    /// this process's CPU and memory usage is written
    pub async fn init(
        path: impl AsRef<Path>,
        min_level: LogLevel,
        perf_interval: Duration,
    ) -> Result<(), InitError> {

        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(InitError::Io)?;
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(InitError::Io)?;

        let (tx, mut rx) = mpsc::unbounded_channel::<LogCommand>();

        SENDER.set(tx).map_err(|_| InitError::AlreadyInitialized)?;
        let _ = MIN_LEVEL.set(min_level);

        let handle = tokio::spawn(async move {
            let mut writer = BufWriter::new(file);
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    LogCommand::Write(line) => {
                        let formatted = format_line(&line);
                        if let Err(e) = writer.write_all(formatted.as_bytes()).await {
                            eprintln!("georust_logger: write failed: {e}");
                            continue;
                        }
                        if let Err(e) = writer.flush().await {
                            eprintln!("georust_logger: flush failed: {e}");
                        }
                    }
                    LogCommand::Flush(ack) => {
                        let _ = writer.flush().await;
                        let _ = ack.send(());
                    }
                }
            }
            let _ = writer.flush().await;
        });

        *TASK_HANDLES.lock().unwrap() = vec![handle];

        let perf_handle = tokio::spawn(performance_monitor_loop(perf_interval));
        TASK_HANDLES.lock().unwrap().push(perf_handle);

        Ok(())
    }

    /// Waits until every line enqueued so far has been written to disk.
    /// Call this before process exit if you want a guarantee that the last
    /// burst of logs made it to the file.
    pub async fn flush() {
        let Some(tx) = SENDER.get() else { return };
        let (ack_tx, ack_rx) = oneshot::channel();
        if tx.send(LogCommand::Flush(ack_tx)).is_ok() {
            let _ = ack_rx.await;
        }
    }
}

#[doc(hidden)]
pub fn __send_line(line: LogLine) {
    match SENDER.get() {
        Some(tx) => {
            // Unbounded channel: this never blocks or awaits, so calling
            // this from sync *or* async code is fine either way.
            let _ = tx.send(LogCommand::Write(line));
        }
        None => {
            // Never panic on a logging call -- a missing Logger::init() call
            // shouldn't be able to crash the server. Fall back to stderr so
            // the line isn't silently lost during development.
            eprintln!(
                "georust_logger: Logger::init() not called yet, dropping line: {:?}",
                line
            );
        }
    }
}

/// One JSON object per line (JSON Lines / ndjson), e.g.:
/// `{"time":"...","level":"debug","module":"main","event":"startup","message":"..."}`
fn format_line(line: &LogLine) -> String {
    match serde_json::to_string(line) {
        Ok(mut json) => {
            json.push('\n');
            json
        }
        Err(e) => {
            format!(
                "{{\"time\":\"{}\",\"level\":\"error\",\"module\":\"main\",\"event\":\"log_serialize_failed\",\"message\":\"{}\"}}\n",
                line.time,
                e.to_string().replace('"', "'")
            )
        }
    }
}

/// Runs for the lifetime of the process, waking up every `interval` to write
/// a `LogModule::Statistics` line with this process's CPU% and RSS memory.
async fn performance_monitor_loop(interval: Duration) {
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let pid = match sysinfo::get_current_pid() {
        Ok(pid) => pid,
        Err(e) => {
            eprintln!("georust_logger: could not resolve current PID, performance monitor disabled: {e}");
            return;
        }
    };

    let mut sys = System::new();
    let pid_slice = [pid];
    sys.refresh_processes(ProcessesToUpdate::Some(&pid_slice), true);

    loop {
        ticker.tick().await;
        sys.refresh_processes(ProcessesToUpdate::Some(&pid_slice), true);

        let Some(process) = sys.process(pid) else {
            continue;
        };

        let cpu_percent = process.cpu_usage(); // % of one core, since last refresh
        let mem_mb = process.memory() as f64 / (1024.0 * 1024.0);

        if is_enabled(LogLevel::Info) {
            __send_line(LogLine {
                time: chrono::Local::now().to_rfc3339(),
                level: LogLevel::Info,
                module: LogModule::Statistics,
                event: "perf_snapshot".to_string(),
                message: format!("cpu={cpu_percent:.2}% mem={mem_mb:.1}MB pid={pid}"),
            });
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log_line {
    ($level:expr, $module:expr, $event:expr, $($arg:tt)*) => {{
        if $crate::is_enabled($level) {
            $crate::__send_line($crate::LogLine {
                time: $crate::chrono::Local::now().to_rfc3339(),
                level: $level,
                module: $module,
                event: ::std::string::ToString::to_string($event),
                message: ::std::format!($($arg)*),
            });
        }
    }};
}

#[macro_export]
macro_rules! info {
    ($module:expr, $event:expr, $($arg:tt)*) => {
        $crate::__log_line!($crate::LogLevel::Info, $module, $event, $($arg)*)
    };
}

#[macro_export]
macro_rules! debug {
    ($module:expr, $event:expr, $($arg:tt)*) => {
        $crate::__log_line!($crate::LogLevel::Debug, $module, $event, $($arg)*)
    };
}

#[macro_export]
macro_rules! warn {
    ($module:expr, $event:expr, $($arg:tt)*) => {
        $crate::__log_line!($crate::LogLevel::Warn, $module, $event, $($arg)*)
    };
}

#[macro_export]
macro_rules! error {
    ($module:expr, $event:expr, $($arg:tt)*) => {
        $crate::__log_line!($crate::LogLevel::Error, $module, $event, $($arg)*)
    };
}