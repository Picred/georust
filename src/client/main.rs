use std::fmt::format;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;

mod coord_gen;
use crate::coord_gen::CoordGenerator;
mod config;
use config::Config;
mod console;
use console::ConsoleEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::load("./config/client_config.txt")
        .map_err(|e| format!("Error while parsing client config file: {} ", e))?;

    let mut generator = CoordGenerator::init(&cfg.coord_file_path)
        .map_err(|e| format!("Error while creating CoordGenerator: {} ", e))?;

    println!("Connecting to {}...", cfg.server_url);
    let (ws_stream, response) = connect_async(&cfg.server_url)
        .await
        .map_err(|e| format!("Failed to connect to {}: {e}", cfg.server_url))?;
    println!("Connected (HTTP status: {})", response.status());

    // placeholder websocket r/w stream
    // let (ws_write, ws_read) = dummy_ws_pair();

    let (ws_write, ws_read) = ws_stream.split();

    // Channel client->server communication
    let (out_tx, out_rx) = mpsc::channel::<Message>(64);

    let (event_tx, mut event_rx) = mpsc::channel::<ConsoleEvent>(8);

    let writer_handle = tokio::spawn(writer_task(ws_write, out_rx));
    let reader_handle = tokio::spawn(reader_task(ws_read));

    let console_out_tx = out_tx.clone();
    let console_handle = tokio::spawn(console::run(console_out_tx, event_tx));

    let mut ticker = time::interval(Duration::from_millis(cfg.tick_interval_millis.into()));
    let mut sending = false;

    let mut logged_in = false;

    println!("Client console succesfully initialized. Type \"help\" for available commands");

    loop {
        tokio::select! {
            _ = ticker.tick() => {

                if !logged_in {
                    let login_json = r#"{"action":"login","username":"veicolo_test1","password":"password_test"}"#;
                    out_tx.send(Message::Text(login_json.into())).await.unwrap();
                    logged_in = true;
                }
                 
                if !sending {
                    continue;
                }

                match generator.get_next() {
                    Some(coord) => {
                        let json = serde_json::to_string(&coord).expect("failed to serialize coord");
                        if out_tx.send(Message::Text(json)).await.is_err() {
                            eprintln!("Outgoing channel closed, stopping.");
                            break;
                        }
                    }
                    None => {
                        println!("No more coordinates to send.");
                        sending = false;
                    }
                }
            }
            Some(event) = event_rx.recv() => {
                if let console::LoopControl::Exit = console::handle_event(event, &mut sending) {
                    break;
                }
            }
        }
    }

    drop(out_tx);

    let _ = writer_handle.await;
    let _ = reader_handle.await;
    console_handle.abort();

    Ok(())
}

async fn writer_task(
    mut ws_write: impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    mut rx: mpsc::Receiver<Message>,
) {
    while let Some(msg) = rx.recv().await {
        if let Err(e) = ws_write.send(msg).await {
            eprintln!("Error sending message: {e}");
            break;
        }
    }
}

async fn reader_task(
    mut ws_read: impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
) {
    while let Some(msg) = ws_read.next().await {
        match msg {
            Ok(Message::Text(text)) => println!("\n[server] {text}"),
            Ok(_) => {}
            Err(e) => {
                eprintln!("WebSocket read error: {e}");
                break;
            }
        }
    }
    println!("Server connection closed.");
}
/*
fn dummy_ws_pair() -> (
    impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
) {
    use futures_util::sink::drain;
    use futures_util::stream::pending;
    use std::convert::Infallible;

    let sink = drain().sink_map_err(|e: Infallible| match e {});

    let stream = pending::<Result<Message, tokio_tungstenite::tungstenite::Error>>();

    (sink, stream)
}
*/
