use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

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
/// `sending` state and prints the appropriate feedback. Returns
/// whether the main loop should keep running or exit.
pub fn handle_event(event: ConsoleEvent, sending: &mut bool) -> LoopControl {
    match event {
        ConsoleEvent::Start => {
            if *sending {
                println!("Already sending.");
            } else {
                println!("Position sending enabled.");
                *sending = true;
            }
            LoopControl::Continue
        }
        ConsoleEvent::Stop => {
            if !*sending {
                println!("Already stopped.");
            } else {
                println!("Position sending paused.");
                *sending = false;
            }
            LoopControl::Continue
        }
        ConsoleEvent::Help => {
            println!("  Available commands:
    - start: start sending position to the server
    - stop: stop sending position to the server
    - help: display this String
    - exit: exit the program");
            LoopControl::Continue
        }
        ConsoleEvent::Exit => {
            println!("Shutdown requested from console. Exiting.");
            LoopControl::Exit
        }
    }
}

/// Reads lines from stdin and dispatches them as commands.
///
/// Supported commands:
/// - `start`       -> begins periodic position sending (via `event_tx`)
/// - `stop`        -> pauses periodic position sending (via `event_tx`)
/// - `help`        -> display command help menu
/// - `exit`        -> triggers shutdown (via `event_tx`)
/// - `send <text>` -> forwards `<text>` to the server (via `out_tx`)
pub async fn run(out_tx: mpsc::Sender<Message>, event_tx: mpsc::Sender<ConsoleEvent>) {
    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();

    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if line == "exit" {
                    let _ = event_tx.send(ConsoleEvent::Exit).await;
                    break;
                } else if line == "start" {
                    if event_tx.send(ConsoleEvent::Start).await.is_err() {
                        eprintln!("Failed to start: control channel closed.");
                        break;
                    }
                } else if line == "stop" {
                    if event_tx.send(ConsoleEvent::Stop).await.is_err() {
                        eprintln!("Failed to stop: control channel closed.");
                        break;
                    }
                } else if line == "help" {
                    if event_tx.send(ConsoleEvent::Help).await.is_err() {
                        eprintln!("Failed to display help menu.");
                        break;
                    }
                } else if let Some(rest) = line.strip_prefix("send ") {
                    let payload = rest.to_string();
                    if out_tx.send(Message::Text(payload)).await.is_err() {
                        eprintln!("Failed to send: channel closed.");
                        break;
                    }
                } else if line == "send" {
                    eprintln!("Usage: send <message>");
                } else {
                    eprintln!("Unknown command: {line}");
                }
            }
            Ok(None) => break, // stdin closed (EOF)
            Err(e) => {
                eprintln!("Error reading stdin: {e}");
                break;
            }
        }
    }
}