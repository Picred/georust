use std::io::Write as _;

use rustyline_async::{Readline, ReadlineError, ReadlineEvent, SharedWriter};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use georust::utils::message;

pub enum ConsoleEvent {
    Start,
    Stop,
    Help,
    Exit,
}

pub enum LoopControl {
    Continue,
    Exit,
}

/// Handles a `ConsoleEvent`, changing (if needed) the main loop's
/// `sending` state and prints the appropriate feedback via `writer`.
/// Returns whether the main loop should keep running or exit.
pub fn handle_event(event: ConsoleEvent, sending: &mut bool, writer: &mut SharedWriter) -> LoopControl {
    match event {
        ConsoleEvent::Start => {
            if *sending {
                let _ = writeln!(writer, "Already sending.");
            } else {
                let _ = writeln!(writer, "Position sending enabled.");
                *sending = true;
            }
            LoopControl::Continue
        }
        ConsoleEvent::Stop => {
            if !*sending {
                let _ = writeln!(writer, "Already stopped.");
            } else {
                let _ = writeln!(writer, "Position sending paused.");
                *sending = false;
            }
            LoopControl::Continue
        }
        ConsoleEvent::Help => {
            let _ = writeln!(
                writer,
                "  Available commands:\n\
                 \t- start: start sending position to the server\n\
                 \t- stop: stop sending position to the server\n\
                 \t- help: display this help menu\n\
                 \t- exit: exit the program"
            );
            LoopControl::Continue
        }
        ConsoleEvent::Exit => {
            let _ = writeln!(writer, "Shutdown requested from console. Exiting.");
            LoopControl::Exit
        }
    }
}

/// Initialises the `rustyline-async` readline and returns both the `Readline`
/// driver and the `SharedWriter` that every task should use instead of
/// `println!` / `eprintln!`.
pub fn init() -> Result<(Readline, SharedWriter), ReadlineError> {
    Readline::new("> ".to_string())
}

/// Reads lines from the rustyline-async `Readline` and dispatches them
/// as commands.
///
/// Supported commands:
/// - `start`       -> begins periodic position sending (via `event_tx`)
/// - `stop`        -> pauses periodic position sending (via `event_tx`)
/// - `help`        -> display command help menu
/// - `exit`        -> triggers shutdown (via `event_tx`)
/// - `send <text>` -> forwards `<text>` to the server (via `out_tx`)
pub async fn run(
    mut rl: Readline,
    mut writer: SharedWriter,
    out_tx: mpsc::Sender<Message>,
    event_tx: mpsc::Sender<ConsoleEvent>,
) {
    loop {
        match rl.readline().await {
            Ok(ReadlineEvent::Line(raw)) => {
                let line = raw.trim().to_string();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line.clone());

                if line == "exit" {
                    // The task will be cancelled by console_handle.abort() in main.
                    let _ = event_tx.send(ConsoleEvent::Exit).await;

                } else if line == "start" {
                    if event_tx.send(ConsoleEvent::Start).await.is_err() {
                        let _ = writeln!(writer, "Failed to start: control channel closed.");
                        break;
                    }
                } else if line == "stop" {
                    if event_tx.send(ConsoleEvent::Stop).await.is_err() {
                        let _ = writeln!(writer, "Failed to stop: control channel closed.");
                        break;
                    }
                } else if line == "help" {
                    if event_tx.send(ConsoleEvent::Help).await.is_err() {
                        let _ = writeln!(writer, "Failed to display help menu.");
                        break;
                    }
                } else if let Some(rest) = line.strip_prefix("send ") {
                    let msg = message::Message { body: rest.to_string() };
                    let msg_json =
                        serde_json::to_string(&msg).expect("failed to serialize message");
                    if out_tx.send(Message::Text(msg_json)).await.is_err() {
                        let _ = writeln!(writer, "Outgoing channel closed, stopping.");
                        break;
                    }
                } else if line == "send" {
                    let _ = writeln!(writer, "Usage: send <message>");
                } else {
                    let _ = writeln!(writer, "Unknown command: {line}");
                }
            }
            // Ctrl-D: ignore
            Ok(ReadlineEvent::Eof) | Err(ReadlineError::Closed) => continue,
            // Ctrl-C: treat as a soft exit request
            Ok(ReadlineEvent::Interrupted) => {
                let _ = event_tx.send(ConsoleEvent::Exit).await;
                break;
            }
            Err(e) => {
                let _ = writeln!(writer, "Error reading input: {e}");
                break;
            }
        }
    }
}
