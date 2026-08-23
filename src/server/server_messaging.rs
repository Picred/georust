use crate::ConnectionManager;
use G19::utils::message::Message;

pub async fn send(command_params: &[&str], manager: &ConnectionManager) {
    if command_params.len() < 3 {
        println!("Uso corretto: send <user_id> <messaggio>");
        return;
    }

    let target_user_id = match command_params[1].parse::<i64>() {
        Ok(id) => id,
        Err(_) => {
            println!("Errore: <user_id> deve essere un numero intero.");
            return;
        }
    };

    let message_body = command_params[2..].join(" ");
    let msg = Message { body: message_body };

    let json_string = match serde_json::to_string(&msg) {
        Ok(json) => json,
        Err(e) => {
            println!("Errore di serializzazione JSON: {}", e);
            return;
        }
    };

    let ws_msg = tokio_tungstenite::tungstenite::Message::Text(json_string.into());
    let sockets_guard = manager.sockets.read().await;
    let mut found = false;

    for socket in sockets_guard.values() {
        if socket.user_id == Some(target_user_id) {
            if socket.tx.send(ws_msg.clone()).await.is_ok() {
                println!("Messaggio inviato all'utente {}", target_user_id);
            } else {
                println!("Errore: canale chiuso per l'utente {}", target_user_id);
            }
            found = true;
            break;
        }
    }

    if !found {
        println!("Utente {} non trovato o non autenticato.", target_user_id);
    }
}

pub async fn broadcast(command_params: &[&str], manager: &ConnectionManager) {
    if command_params.len() < 2 {
        println!("Uso corretto: broadcast <messaggio>");
        return;
    }

    let message_body = command_params[1..].join(" ");
    let msg = Message { body: message_body };

    let json_string = match serde_json::to_string(&msg) {
        Ok(json) => json,
        Err(e) => {
            println!("Errore di serializzazione JSON: {}", e);
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

    println!("Messaggio in broadcast inviato a {} utenti attivi.", count);
}
