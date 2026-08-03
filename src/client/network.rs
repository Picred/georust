use std::{thread, time};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream};
use tokio::net::TcpStream;
use futures_util::SinkExt;

use crate::client::config::Config;
use crate::client::coord_gen::CoordGenerator;

#[derive(Serialize, Deserialize, Debug)]
pub struct TelemetryData {
    pub lat: f64,
    pub lon: f64,
    pub pos_time: i64, 
}

pub struct ClientNetwork {
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl ClientNetwork {
    pub fn connect(addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let url = format!("ws://{}", addr);
        
        let rt = tokio::runtime::Handle::current();
        let (ws_stream, _) = rt.block_on(connect_async(url))?;
        
        println!("[CLIENT WS] Connessione WebSocket stabilita con ws://{}", addr);
        Ok(Self { ws_stream })
    }

    pub fn authenticate(&mut self, cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
        let login_payload = format!(
            r#"{{"action":"login", "username":"{}", "password":"{}"}}"#,
            cfg.client_username, cfg.client_password
        );

        let rt = tokio::runtime::Handle::current();
        rt.block_on(self.ws_stream.send(Message::Text(login_payload)))?;
        
        println!("[CLIENT WS] Inviate credenziali di autenticazione (JSON)");
        Ok(())
    }

    pub fn start_streaming(
        &mut self,
        generator: &mut CoordGenerator,
        tick_millis: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rt = tokio::runtime::Handle::current();

        while let Some(coord) = generator.get_next() {
            let pos_time = chrono::Utc::now().timestamp_millis();

            let telemetry = TelemetryData {
                lat: coord.0,
                lon: coord.1,
                pos_time,
            };

            let binary_data = bincode::serialize(&telemetry)?;

            rt.block_on(self.ws_stream.send(Message::Binary(binary_data)))?;
            println!(
                "[CLIENT WS] Inviata telemetria BINCODE: lat={}, lon={}, time={}",
                coord.0, coord.1, pos_time
            );

            thread::sleep(time::Duration::from_millis(tick_millis.into()));
        }

        Ok(())
    }
}