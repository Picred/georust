use tokio_tungstenite::tungstenite::Message;
use tokio::net::TcpStream;
use futures_util::StreamExt;
use super::user_state_handler::UserStateHandler;
use super::repository::server_state::ServerState;
use G19::utils::coordinates::Coordinates;
use tokio::sync::mpsc;
use std::time::Duration; // Necessario per definire l'intervallo di tempo
use super::utils::convert_sql_to_naive_datetime;
use G19::utils;

/// Handles the user session by managing:
/// - ping/pong between client and server (in case the server doesn't receice any message or pong before the timeout, it closes the session)
/// - coordinates receiving and storing
/// - creation and call of UserStateHandler
/// - client text message receiving
/// - close or STOP command to end the session
pub async fn handle_user_session(
    ws_receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
    tx: mpsc::Sender<Message>,
    user_id: i64,
    state: &ServerState,
) -> Result<(), Box<dyn std::error::Error>> {

    println!("Avviato loop di tracking con Ping attivo per veicolo (User ID: {})", user_id);

    // Set up a timer that ticks every 10 seconds to send the Ping
    let mut ping_interval = tokio::time::interval(Duration::from_secs(10));
    // Prevent accumulated ticks from firing all at once if the server slows down
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Flag to check if the client has responded to the last sent Ping
    let mut waiting_for_pong = false;

    let mut user_state_handler = UserStateHandler::new();

    loop {
        tokio::select! {
            // CASE 1: The 10-second interval ticks
            _ = ping_interval.tick() => {
                if waiting_for_pong {
                    // If the timer ticks again and the vehicle has not responded to the previous Pong,
                    // the connection is considered dead or unstable (e.g., tunnel or signal loss).
                    println!("Timeout! Il veicolo {} non ha risposto al Pong. Chiudo connessione.", user_id);
                    break;
                }

                // Send the Ping message to the client via the tx channel
                if tx.send(Message::Ping(vec![])).await.is_err() {
                    break; // The write channel is closed, exit the loop
                }
                waiting_for_pong = true;
            }

            // CASE 2: A packet arrives from the client
            maybe_msg = ws_receiver.next() => {

                let msg = match maybe_msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        eprintln!("Errore di rete dal veicolo {}: {:?}", user_id, e);
                        break;
                    }
                    None => {
                        println!("Il flusso dati del veicolo {} si è interrotto bruscamente.", user_id);
                        break;
                    }
                };

                // Intercept the client's Pong response to confirm stability
                if msg.is_pong() {
                    waiting_for_pong = false;
                    continue;
                }

                // Handle explicit close frames sent by the client
                if msg.is_close() {
                    println!("Lo user {} ha chiuso la sessione con un messaggio di CLOSE", user_id);
                    break;
                }

                // Process text messages
                if msg.is_text() {
                    let text = msg.to_text().unwrap_or("");
                    
                    // Immediate check for the STOP command
                    if text == "STOP" {
                        let _ = tx.send(Message::Text("Tracking interrotto con successo.".into())).await;
                        break; // Exits the loop; execution returns to handle_connection for cleanup
                    }

                    // Parsing and inserting coordinates into the SQLite database
                    if let Ok(coords) = serde_json::from_str::<Coordinates>(text) {

                        // Validate the format of the string containing the created_at datetime
                        if convert_sql_to_naive_datetime(coords.created_at.clone()).is_ok() {

                            // Determine user's state
                            let user_state = user_state_handler.calculate_user_state(coords.clone());
    
                            // Insert coordinates into the DB as a journey_waypoint
                            state.journeys_repo.insert_journey_waypoint(user_id, coords.lat, coords.lon, coords.created_at.clone(), user_state).await?;
                            println!("Inserito nel DB: {}, {}, {}, {}, {}", user_id, coords.lat, coords.lon, coords.created_at, user_state);
                        }

                        // Receiving valid data from the vehicle proves it is active, so
                        // clear the pong alert status upon receiving new coordinates as well.
                        waiting_for_pong = false;

                    } else if let Ok(msg) = serde_json::from_str::<utils::message::Message>(text) {
                        // Log the message
                        println!("Ricevuto messaggio: {}", msg.body);
                    } else {
                        let _ = tx.send(Message::Text("{\"error\":\"Formato messaggio non valido o non riconosciuto\"}".into())).await;
                    }
                }
            }
        }
    }

    Ok(())
}





