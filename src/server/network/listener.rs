use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
use futures_util::StreamExt;

use crate::server::repository::{
    journeys_repository::JourneysRepository,
    users_repository::{AuthenticationStatus, UsersRepository},
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TelemetryData {
    pub lat: f64,
    pub lon: f64,
    pub pos_time: i64,
}

pub struct ServerListener {
    users_repo: Arc<UsersRepository>,
    journeys_repo: Arc<JourneysRepository>,
}

impl ServerListener {
    pub fn new(users_repo: Arc<UsersRepository>, journeys_repo: Arc<JourneysRepository>) -> Self {
        Self {
            users_repo,
            journeys_repo,
        }
    }

    pub async fn listen(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        println!("[SERVER WS] In ascolto su ws://{}", addr);

        while let Ok((stream, _)) = listener.accept().await {
            let u_repo = Arc::clone(&self.users_repo);
            let j_repo = Arc::clone(&self.journeys_repo);

            tokio::spawn(async move {
                let ws_stream = match accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        eprintln!("[SERVER WS ERROR] Errore handshake WebSocket: {}", e);
                        return;
                    }
                };

                let (_, mut read) = ws_stream.split();
                let mut authenticated_user_id: Option<i32> = None;

                while let Some(msg_result) = read.next().await {
                    match msg_result {
                        Ok(Message::Text(text_msg)) => {
                            let json: serde_json::Value = match serde_json::from_str(&text_msg) {
                                Ok(val) => val,
                                Err(_) => continue,
                            };

                            if let Some(action) = json.get("action").and_then(|a| a.as_str()) {
                                let username = json["username"].as_str().unwrap_or_default();
                                let password = json["password"].as_str().unwrap_or_default();

                                if action == "login" {
                                    if let Ok(AuthenticationStatus::Success) = u_repo
                                        .validate_user_credentials(username, password.as_bytes())
                                        .await
                                    {
                                        println!("[SERVER WS] Login OK: {}", username);
                                        authenticated_user_id = Some(1); 
                                    }
                                }
                            }
                        }
                        Ok(Message::Binary(bin_msg)) => {
                            if let Ok(telemetry) = bincode::deserialize::<TelemetryData>(&bin_msg) {
                                if let Some(user_id) = authenticated_user_id {
                                    let _ = j_repo
                                        .insert_journey(user_id, telemetry.lat, telemetry.lon)
                                        .await;
                                    println!(
                                        "[SERVER TELEMETRY BINCODE] Lat: {}, Lon: {}, Time: {}",
                                        telemetry.lat, telemetry.lon, telemetry.pos_time
                                    );
                                }
                            }
                        }
                        Ok(Message::Close(_)) => break,
                        Err(e) => {
                            eprintln!("[SERVER WS ERROR]: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            });
        }
        Ok(())
    }
}