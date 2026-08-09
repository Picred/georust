use tokio_tungstenite::tungstenite::Message;
use tokio::net::TcpStream;
use futures_util::StreamExt;
use super::repository::server_state::ServerState;
use G19::utils::coordinates::Coordinates;
use tokio::sync::mpsc;
use std::time::Duration; // Necessario per definire l'intervallo di tempo

pub async fn handle_journey_tracking(
    ws_receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
    tx: mpsc::Sender<Message>,
    user_id: i64,
    state: &ServerState,
) -> Result<(), Box<dyn std::error::Error>> {

    println!("Avviato loop di tracking con Ping attivo per veicolo (User ID: {})", user_id);

    // Configura un timer che scatta ogni 10 secondi per inviare il Ping
    let mut ping_interval = tokio::time::interval(Duration::from_secs(10));
    // Evita che i tick accumulati scattino tutti insieme se il server rallenta
    ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Flag per verificare se il client ha risposto all'ultimo Ping inviato
    let mut waiting_for_pong = false;

    loop {
        tokio::select! {
            // CASO 1: in cui scatta l'intervallo dei 10 secondi senza pong di arrivo dal client oppure in cui si manda un nuovo ping
            _ = ping_interval.tick() => {
                if waiting_for_pong {
                    // Se il timer scatta di nuovo e il veicolo non ha risposto al Pong precedente,
                    // la connessione è considerata morta o instabile (es. galleria o assenza di segnale).
                    println!("Timeout! Il veicolo {} non ha risposto al Pong. Chiudo connessione.", user_id);
                    break;
                }

                // Invia il messaggio di Ping al client tramite il canale tx
                if tx.send(Message::Ping(vec![])).await.is_err() {
                    break; // Il canale di scrittura è chiuso, usciamo
                }
                waiting_for_pong = true;
            }

            // CASO 2: Arriva un pacchetto dal client 
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

                // Intercetta la risposta di Pong del client per confermare la stabilità
                if msg.is_pong() {
                    waiting_for_pong = false;
                    continue;
                }

                // Gestione dei frame di chiusura espliciti inviati dal client
                if msg.is_close() {
                    println!("Lo user {} ha chiuso la sessione in modo pulito.", user_id);
                    break;
                }

                // Elaborazione dei messaggi di testo (Coordinate o STOP)
                if msg.is_text() {
                    let text = msg.to_text().unwrap_or("");
                    
                    // Controllo immediato del comando di STOP
                    if text == "STOP" {
                        let _ = tx.send(Message::Text("Tracking interrotto con successo.".into())).await;
                        break; // Esce dal loop, l'esecuzione tornerà su handle_connection per il cleanup
                    }

                    // Parsing ed inserimento delle coordinate nel database SQLite
                    if let Ok(coords) = serde_json::from_str::<Coordinates>(text) {
                        // TODO (implementazione dell'assegnazione dello stato ai journey_waypoint)
                        state.journeys_repo.insert_journey_waypoint(user_id, coords.lat, coords.lon, coords.created_at.clone(), false).await?;
                        println!("inserito nel db: {}, {}, {}, {}", user_id, coords.lat, coords.lon, coords.created_at);

                        // Opzionale: ricevere dati validi dal veicolo dimostra che è attivo,
                        // quindi azzerare l'allerta del pong anche alla ricezione di coordinate fresche.
                        waiting_for_pong = false;
                    } else {
                        let _ = tx.send(Message::Text("{\"error\":\"Invia coordinate o 'STOP'\"}".into())).await;
                    }
                }
            }
        }
    }

    Ok(())
}



