use repository::journeys_repository::JourneysRepository;
use utils::models::coordinates::Coordinates;


pub async fn handle_journey_tracking(
    &self,
    ws_receiver: &mut futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<TcpStream>>,
    tx: mpsc::Sender<Message>,
    user_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {

    // Creo nuovo journey
    let journey_id = self.state.journeys_repo.new_journey().await;
    // Leggo le coordinate
    while let Some(result) = ws_receiver.next().await {
        let msg = result?;

        if msg.is_text() {
            let text = msg.to_text().unwrap_or("");
            // Controllo immediato del comando di STOP
            if text == "STOP" {
                let _ = tx.send(Message::Text("Tracking interrotto con successo.".into())).await;
                break; // Esce dal loop dedicando, l'esecuzione tornerà su handle_connection per il cleanup
            }

            // Parsing ed inserimento delle coordinate
            if let Ok(coords) = serde_json::from_str::<Coordinates>(text) {
                self.state.journeys_repo.insert_journey_waypoint(journey_id, user_id, coords.lat, coords.lon, coords.pos_time).await;
            } else {
                let _ = tx.send(Message::Text("{\"error\":\"Invia coordinate o 'STOP'\"}".into())).await;
            }
        }
    }
    Ok(())
}


