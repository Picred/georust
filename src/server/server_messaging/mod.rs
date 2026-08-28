use crate::ConnectionManager;
use G19::utils::message::Message;

pub async fn send(command_params: &[&str], manager: &ConnectionManager) {
    if command_params.len() < 3 {
        println!("[MESSAGES] Correct usage: send <user_id> <message>");
        return;
    }

    let target_user_id = match command_params[1].parse::<i64>() {
        Ok(id) => id,
        Err(_) => {
            println!("[MESSAGES] Error: <user_id> must be an integer.");
            return;
        }
    };

    let message_body = command_params[2..].join(" ");
    let msg = Message { body: message_body };

    let json_string = match serde_json::to_string(&msg) {
        Ok(json) => json,
        Err(e) => {
            println!("[MESSAGES] JSON serialization error: {}", e);
            return;
        }
    };

    let ws_msg = tokio_tungstenite::tungstenite::Message::Text(json_string.into());
    let sockets_guard = manager.sockets.read().await;
    let mut found = false;

    for socket in sockets_guard.values() {
        if socket.user_id == Some(target_user_id) {
            if socket.tx.send(ws_msg.clone()).await.is_ok() {
                println!("[MESSAGES] Message sent to user {}", target_user_id);
            } else {
                println!("[MESSAGES] Error: channel closed for user {}", target_user_id);
            }
            found = true;
            break;
        }
    }

    if !found {
        println!("[MESSAGES] User {} not found or not authenticated.", target_user_id);
    }
}

pub async fn broadcast(command_params: &[&str], manager: &ConnectionManager) {
    if command_params.len() < 2 {
        println!("[MESSAGES] Correct usage: broadcast <message>");
        return;
    }

    let message_body = command_params[1..].join(" ");
    let msg = Message { body: message_body };

    let json_string = match serde_json::to_string(&msg) {
        Ok(json) => json,
        Err(e) => {
            println!("[MESSAGES] JSON serialization error: {}", e);
            return;
        }
    };

    let ws_msg = tokio_tungstenite::tungstenite::Message::Text(json_string.into());
    let sockets_guard = manager.sockets.read().await;
    let mut count = 0;

    for socket in sockets_guard.values() {
        if socket.user_id.is_some() {
            if socket.tx.send(ws_msg.clone()).await.is_ok() {
                count += 1;
            }
        }
    }

    println!("[MESSAGES] Broadcast message sent to {} active users.", count);
}

