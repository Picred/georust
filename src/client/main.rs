use std::io::Write as _;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;
use clap::Parser;

mod coord_gen;
mod config;
mod console;
mod auth;

use crate::config::Cli;
use crate::coord_gen::CoordGenerator;
use config::Config;
use console::ConsoleEvent;
use auth::authenticate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let cli = Cli::parse();

    let cfg = Config::load(&cli)
        .map_err(|e| format!("Error while parsing client config file: {} ", e))?;

    let mut generator = CoordGenerator::init(&cfg.coord_file_path)
        .map_err(|e| format!("Error while creating CoordGenerator: {} ", e))?;

    // Initialise rustyline-async: all output must go through `writer` so that
    // in-progress user input is never garbled by concurrent prints.
    let (rl, mut writer) = console::init()
        .map_err(|e| format!("Failed to initialise console: {e}"))?;

    writeln!(writer, "Connecting to {}...", cfg.server_url)?;
    let (ws_stream, response) = connect_async(&cfg.server_url)
        .await
        .map_err(|e| format!("Failed to connect to {}: {e}", cfg.server_url))?;
    writeln!(writer, "Connected (HTTP status: {})", response.status())?;

    let (mut ws_write, mut ws_read) = ws_stream.split();

    let _ = authenticate(&cfg, &mut ws_write, &mut ws_read)
        .await
        .map_err(|e| format!("Authentication failed: {e}"))?;
    writeln!(writer, "Authenticated successfully")?;

    // Channel client->server communication
    let (out_tx, out_rx) = mpsc::channel::<Message>(64);
    let (event_tx, mut event_rx) = mpsc::channel::<ConsoleEvent>(8);

    let writer_handle = tokio::spawn(writer_task(ws_write, out_rx));

    // Give reader_task its own clone of writer so it can print server messages.
    let reader_writer = writer.clone();
    let reader_handle = tokio::spawn(reader_task(ws_read, reader_writer));

    // Give console::run its own clone for command feedback.
    let console_writer = writer.clone();
    let console_out_tx = out_tx.clone();
    let console_handle = tokio::spawn(console::run(rl, console_writer, console_out_tx, event_tx));

    let mut ticker = time::interval(Duration::from_millis(cfg.tick_interval_millis.into()));
    let mut sending = true;

    writeln!(writer, "Client console successfully initialized. Type \"help\" for available commands")?;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                 
                if !sending {
                    continue;
                }

                match generator.get_next() {
                    Some(coord) => {
                        let json = serde_json::to_string(&coord).expect("failed to serialize coord");
                        if out_tx.send(Message::Text(json)).await.is_err() {
                            let _ = writeln!(writer, "Outgoing channel closed, stopping.");
                            break;
                        }
                    }
                    None => {
                        let _ = writeln!(writer, "No more coordinates to send.");
                        sending = false;
                    }
                }
            }
            Some(event) = event_rx.recv() => {
                if let console::LoopControl::Exit = console::handle_event(event, &mut sending, &mut writer) {
                    break;
                }
            }
            _ = tokio::signal::ctrl_c() => {
                let _ = writeln!(writer, "Ctrl-C received, exiting...");
                break;
            }
        }
    }

    // Inform the server with a Close message before dropping the websocket stream
    let _ = out_tx.send(Message::Close(None)).await;
    drop(out_tx);

    console_handle.abort();
    let _ = writer_handle.await;
    reader_handle.abort();


    Ok(())
}

async fn writer_task(
    mut ws_write: impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    mut rx: mpsc::Receiver<Message>,
) {
    while let Some(msg) = rx.recv().await {
        let is_close = msg.is_close();
        if let Err(e) = ws_write.send(msg).await {
            eprintln!("Error sending message: {e}");
            break;
        }
        if is_close {
            break;
        }
    }
}

async fn reader_task(
    mut ws_read: impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    mut writer: rustyline_async::SharedWriter,
) {
    while let Some(msg) = ws_read.next().await {
        match msg {
            Ok(Message::Text(json)) => {
                let _ = writeln!(writer, "[server] {json}");
            },
            Ok(Message::Close(_)) => {
                break;
            }
            Ok(_) => {}
            Err(e) => {
                let _ = writeln!(writer, "WebSocket read error: {e}");
                break;
            }
        }
    }
    let _ = writeln!(writer, "Server connection closed.");
}

