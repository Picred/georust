use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

use crate::config;
use config::Config;

#[derive(Debug, Deserialize)]
struct AuthResponse {
    status: String,
    message: String,
    user_id: Option<i64>
}

/// Logs into the server using `cfg.client_username` / `cfg.client_password`.
/// If login fails, registers those credentials, then logs in again.
pub async fn authenticate(
    cfg: &Config,
    ws_write: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    ws_read: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> Result<i64, Box<dyn std::error::Error>> {

    let response = login(cfg, ws_write, ws_read).await?;
    if response.status == "success" {
        println!("Login successful: {}", response.message);
        return response
            .user_id
            .ok_or_else(|| "Login reported success but no user_id was returned".into());
    }
    println!("Login failed ({}), attempting registration...", response.message);

    let response = register(cfg, ws_write, ws_read).await?;
    if response.status != "success" {
        return Err(format!("Registration failed, another user with the same username exists: {}", response.message).into());
    }
    println!("Registration successful: {}", response.message);

    println!("Retrying login after registration...");
    let response = login(cfg, ws_write, ws_read).await?;
    if response.status == "success" {
        println!("Login successful: {}", response.message);
        response
            .user_id
            .ok_or_else(|| "Login reported success but no user_id was returned".into())
    } else {
        Err(format!("Login after registration failed: {}", response.message).into())
    }
}

/// Attempts to log in with the given credentials. Returns the parsed
/// `AuthResponse` regardless of success/failure - the caller decides
/// what to do next based on `response.status`.
async fn login(
    cfg: &Config,
    ws_write: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    ws_read: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> Result<AuthResponse, Box<dyn std::error::Error>> {
    let payload = json!({
        "action": "login",
        "username": cfg.client_username,
        "password": cfg.client_password,
    });
    ws_write.send(Message::Text(payload.to_string())).await?;
    next_auth_response(ws_read).await
}

/// Attempts to register the given credentials. Returns the parsed
/// `AuthResponse` - a successful registration does not means the
/// client is logged in; the caller must call `login` again afterward.
async fn register(
    cfg: &Config,
    ws_write: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    ws_read: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> Result<AuthResponse, Box<dyn std::error::Error>> {
    let payload = json!({
        "action": "register",
        "username": cfg.client_username,
        "password": cfg.client_password,
    });
    ws_write.send(Message::Text(payload.to_string())).await?;
    next_auth_response(ws_read).await
}

/// Waits for the next text message from the server and parses it as an `AuthResponse`.
async fn next_auth_response(
    ws_read: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> Result<AuthResponse, Box<dyn std::error::Error>> {
    loop {
        match ws_read.next().await {
            Some(Ok(Message::Text(text))) => return Ok(serde_json::from_str(&text)?),
            Some(Ok(_)) => continue, // ignore other messages while waiting for the auth reply
            Some(Err(e)) => return Err(format!("WebSocket error during auth: {e}").into()),
            None => return Err("Connection closed before receiving a response".into()),
        }
    }
}