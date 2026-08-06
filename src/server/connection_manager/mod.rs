/*
Modulo in cui si gestisce la connessione tramite WebSocket con l'ausilio di Tunstenite.
Il sistema si divide in:
    - TASK DISPATCER:
        - aspetta che venga creata una connessione TCP
        - genera codice univoco del socket per la connessione
        - genera task del connession-handler
    - TASK CONNECTION-HANDLER:
        - esegue handshake del WebSocket
        - splitta il WebSocket in:
            - ws_receiver: riceve i messaggi inviati dal client verso il server
            - ws_sender: invia i messaggi dal server verso il client
        - inserisce il socket nella mappa sockets del ConnectionManger
        - spawna il micro-task di scrittura verso il client (si attiva quando c'è un msg generato da un task del server nel canale)
        - loop di ricezione messaggi dal client:
            - fase di LOGIN/REGISTER
            - fase di TRACKING:
                    - invio coordinate
        - disconnessione
 */


use std::collections::HashMap;  
use std::sync::Arc;
use std::time::Duration;
use sqlx::{Pool, Sqlite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;
use super::repository::server_state::ServerState;
use serde::{Deserialize, Serialize};
use super::repository::users_repository::AuthenticationStatus;
use super::journey_tracking::handle_journey_tracking;

// struct per la ocmunicazione in fase di login/register
#[derive(Deserialize)]
struct AuthRequest {
    action: String, // "login" o "register"
    username: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    status: String,
    message: String,
    user_id: Option<i64>,
}

// La struttura che rappresenta il socket in memoria
pub struct ActiveSocket {
    pub socket_id: Uuid,         // Il codice univoco generato al momento della connessione
    pub user_id: Option<i64>,     // è None prima del login
    pub tx: mpsc::Sender<Message>, // tx da clonare in un task per trasmettere al ws_sender (gestito da un task in ascolto su rx) per poi comunicare con il client
}

pub struct ConnectionManager {
    // mappa dei socket per collegare ogni ActiveSocket a un codice univoco socket_id (di tipo Uuid)
    pub sockets: Arc<RwLock<HashMap<Uuid, ActiveSocket>>>,
    pub state: Arc<ServerState>,
}

impl ConnectionManager {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self {
            sockets: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(ServerState::new(pool)),
        }
    }

    // ottenere user_id partendo da socked_id
    pub async fn get_user_id_by_socket_id(&self, socket_id: &Uuid) -> Option<i64> {
        let guard = self.sockets.read().await;
        guard.get(socket_id).and_then(|socket| socket.user_id)
    }

    // TASK DISPATCHER
    pub async fn run(self: Arc<Self>, listener: TcpListener) {
        println!("Server in ascolto...");
        while let Ok((stream, _)) = listener.accept().await {   // si stabiliste la connessione TCP
            let manager = self.clone();
            
            // si genera il codice univoco del socket legato alla connessione stabilita
            let socket_id = Uuid::new_v4();
            println!("Nuova connessione TCP accettata. Assegnato Socket ID: {}", socket_id);

            // si passa crea il task del connection_handler
            tokio::spawn(async move {
                if let Err(e) = manager.handle_connection(stream, socket_id).await {
                    eprintln!("Errore nella gestione della connessione [{}]: {:?}", socket_id, e);
                }
            });
        }
    }

    // TASK CONNECTION-HANDLER
    async fn handle_connection(&self, stream: TcpStream, socket_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
        let ws_stream = accept_async(stream).await?;  // esegue handshake del WebSocket

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Creazione del canale di comunicazione interno al server per questo socket
        let (tx, mut rx) = mpsc::channel::<Message>(100);

        // Inserimento del socket nella mappa globale (con user_id = None perchè questo non si è ancora autenticato)
        {
            let active_socket = ActiveSocket {
                socket_id,
                user_id: None, 
                tx: tx.clone(),
            };
            self.sockets.write().await.insert(socket_id, active_socket);
        }

        // Spawn del micro-task di scrittura verso il client che si attiva quando c'è un msg generato dal server nel canale
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Parte per la ricezione dal client
        let mut autenticato_user_id: Option<i64> = None;

        // ==========================================================
        // FASE DI AUTENTICAZIONE (Gira finché l'utente non è loggato)
        // ==========================================================
        while let Some(result) = ws_receiver.next().await {
            let msg = result?;

            // GESTIONE PING da parte del Client (se necessaria)
            /*if msg.is_ping() {
                // Risponde al ping rimandando i dati ricevuti nel payload del ping (richiesto dallo standard)
                let _ = tx.send(Message::Pong(msg.into_data())).await;
                continue;
            }*/

            // GESTIONE CLOSE (il client disconnettendosi prima dell'autenticazione invia un frame di chiusura, 
            // in questo modo viene interrotta la fase di autenticazione)
            if let Message::Close(frame) = &msg {
                if let Some(cf) = frame {
                    println!("Client disconnesso durante l'auth. Codice: {}, Motivo: {}", cf.code, cf.reason);
                } else {
                    println!("Client disconnesso durante l'auth senza dettagli.");
                }
                break; 
            }

            // GESTIONE TESTO (JSON)
            if msg.is_text() {
                let text = msg.to_text().unwrap_or("");
                
                let auth_data: AuthRequest = match serde_json::from_str(text) {
                    Ok(data) => data,
                    Err(_) => {

                        // Crea risposta di errore (AuthResponse) da trasmettere al client
                        let err_resp = AuthResponse {
                            status: "error".into(),
                            message: "JSON non valido. Invia le credenziali per accedere.".into(),
                            user_id: None,
                        };

                        // Trasforma in stringa e trasmette l'errore al client
                        if let Ok(json) = serde_json::to_string(&err_resp) {
                            let _ = tx.send(Message::Text(json.into())).await;
                        }
                        continue; // Salta al prossimo messaggio senza uscire dal loop
                    }
                };

                match auth_data.action.as_str() { // controlla se valore di "action": "..."

                    // CASO LOGIN
                    "login" => {
                        match self.state.users_repo.validate_user_credentials(&auth_data.username, auth_data.password.as_bytes()).await {

                            // Caso di SUCCESSO
                            Ok(AuthenticationStatus::Success(id)) => {
                                autenticato_user_id = Some(id);
                                
                                // Sincronizza l'ID verificato nella mappa globale
                                {
                                    let mut guard = self.sockets.write().await;
                                    if let Some(socket) = guard.get_mut(&socket_id) {
                                        socket.user_id = Some(id);
                                    }
                                }

                                // Crea e spedisce risposta al client
                                let resp = AuthResponse {
                                    status: "success".into(),
                                    message: format!("Login effettuato! Benvenuto {}", auth_data.username),
                                    user_id: Some(id),
                                };
                                if let Ok(json) = serde_json::to_string(&resp) {
                                    let _ = tx.send(Message::Text(json.into())).await;
                                }

                                break; // Interrompe il loop di autenticazione per proseguire alla FASE DI ATTESA SELEZIONE MODALITÀ
                            }

                            // Caso di INSUCESSO per CREDENTIALI NON VALIDE
                            Ok(AuthenticationStatus::InvalidCredentials) => {
                                let resp = AuthResponse { status: "error".into(), message: "Username o password errati.".into(), user_id: None };
                                if let Ok(json) = serde_json::to_string(&resp) { let _ = tx.send(Message::Text(json.into())).await; }
                            }

                            // Caso di INSUCCESSO per ERRORE del DB
                            Err(e) => {
                                let resp = AuthResponse { status: "error".into(), message: format!("Errore DB: {}", e), user_id: None };
                                if let Ok(json) = serde_json::to_string(&resp) { let _ = tx.send(Message::Text(json.into())).await; }
                            }
                        }
                    }


                    // CASO REGISTRAZIONE
                    "register" => {
                        match self.state.users_repo.insert_user(&auth_data.username, auth_data.password.as_bytes()).await {

                            // Caso di SUCCESSO
                            Ok(id) => {
                                let resp = AuthResponse {
                                    status: "success".into(),
                                    message: "Registrazione completata! Ora effettua il login.".into(),
                                    user_id: Some(id),
                                };
                                if let Ok(json) = serde_json::to_string(&resp) {
                                    let _ = tx.send(Message::Text(json.into())).await;
                                }
                                // Qui NON viene messo il break perchè l'utente si è solo registrato,
                                // il loop continua per permettergli di fare il "login".
                            }

                            // Caso di INSUCCESSO per ERRORE del DB
                            Err(e) => {
                                let resp = AuthResponse {
                                    status: "error".into(),
                                    message: format!("Errore registrazione (es. utente esistente): {:?}", e),
                                    user_id: None,
                                };
                                if let Ok(json) = serde_json::to_string(&resp) {
                                    let _ = tx.send(Message::Text(json.into())).await;
                                }
                            }
                        }
                    }

                    // CASO INDEFINITO
                    _ => {
                        let resp = AuthResponse { status: "error".into(), message: "Usa 'login' o 'register'.".into(), user_id: None };
                        if let Ok(json) = serde_json::to_string(&resp) { let _ = tx.send(Message::Text(json.into())).await; }
                    }
                }
            }
        }

        // Controllo di sicurezza: se il client chiude la connessione bruscamente 
        // prima di loggarsi con successo o di inviare un frame di chiusura,
        // si esce anticipatamente eseguendo il cleanup.
        let user_id = match autenticato_user_id {
            Some(id) => id,
            None => {
                self.sockets.write().await.remove(&socket_id);
                return Ok(());
            }
        };

        // ==========================================================
        // FASE DI TRACKING CON INVIO PING ATTIVO E VERIFICA TIMEOUT
        // ==========================================================

        let _ = tx.send(Message::Text("{\"status\":\"success\",\"message\":\"Modalità Tracking avviata. Invia coordinate o 'STOP'\"}".into())).await;
        // AVVIO del task per il tracking (per ricezione ed inserimento delle coordinate nel db)
        // e gestione del PING per controllare stabilità della connessione con il client
        handle_journey_tracking(&mut ws_receiver, tx.clone(), user_id, &self.state).await?;

        // CLEANUP AL DISCONNECT
        self.sockets.write().await.remove(&socket_id);
        println!("Socket [{}] rimosso dalla mappa globale causa disconnessione.", socket_id);

        Ok(())
    }
}




// ==========================================================
// BLOCCO DI TEST DI INTEGRAZIONE (per testare connection manager e handle_journey_tracking)
// ==========================================================
#[cfg(test)]
mod tests {

    use super::*;
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    use futures_util::{StreamExt, SinkExt};
    use sqlx::SqlitePool;
    use std::time::Duration;

    #[tokio::test]
    async fn test_user_tracking_successful_flow() {
        // Inizializzazione DB SQLite temporaneo in memoria isolato per il test
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        
        // Crea le tabelle necessarie per il test
        sqlx::query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username TEXT, password BLOB);")
            .execute(&pool).await.unwrap();
        /*sqlx::query("INSERT INTO users (id, username, password) VALUES (1, 'veicolo_test', 'password_test');")
            .execute(&pool).await.unwrap();*/
        sqlx::query("CREATE TABLE IF NOT EXISTS journeys (id INTEGER PRIMARY KEY, user_id INTEGER, lat REAL, lon REAL, created_at TEXT);")
            .execute(&pool).await.unwrap();

        // Avvia il ConnectionManager su una porta casuale libera (porta 0)
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();
        
        let manager = Arc::new(ConnectionManager::new(pool.clone()));
        let manager_clone = manager.clone();

        // Avvia il server in un task di background dedicato
        tokio::spawn(async move {
            manager_clone.run(listener).await;
        });


        // ==========================================================
        // SIMULAZIONE CLIENT
        // ==========================================================

        // Avvio delle connessione con il server
        let url = format!("ws://{}", local_addr);
        let (ws_stream, _) = connect_async(&url).await.expect("Connessione fallita");
        let (mut client_tx, mut client_rx) = ws_stream.split();


        // TEST REGISTER
        let login_json = r#"{"action":"register","username":"veicolo_test","password":"password_test"}"#;
        client_tx.send(Message::Text(login_json.into())).await.unwrap();

        // Aspetta la risposta dal server
        if let Some(Ok(Message::Text(response))) = client_rx.next().await {
            assert!(response.contains("success"), "La registrazione è fallita: {}", response);
            assert!(response.contains("Registrazione completata!"), "Messaggio di conferma della registrazione errato");
        } else {
            panic!("Non è stata ricevuta una risposta di testo valida per la registrazione");
        }


        // TEST LOGIN
        let login_json = r#"{"action":"login","username":"veicolo_test","password":"password_test"}"#;
        client_tx.send(Message::Text(login_json.into())).await.unwrap();

        // Aspetta la risposta dal server
        if let Some(Ok(Message::Text(response))) = client_rx.next().await {
            assert!(response.contains("success"), "Il login è fallito: {}", response);
            assert!(response.contains("Login effettuato!"), "Messaggio di benvenuto errato");
        } else {
            panic!("Non è stata ricevuta una risposta di testo valida per il login");
        }

        // TEST AVVIO TRACKING & PING ATTIVO
        // Il server invia un messaggio di avvio modalità tracking subito dopo il login
        if let Some(Ok(Message::Text(response))) = client_rx.next().await {
            assert!(response.contains("Modalità Tracking avviata"), "Mancato avviso avvio tracking");
        }

        // Ora testiamo se il server ci manda il PING attivo (impostato a 10 secondi nel server)
        // Usiamo un timeout sul test per non rimanere appesi se il server fallisce
        let msg_dal_server = tokio::time::timeout(Duration::from_secs(12), client_rx.next()).await
            .expect("Il server non ha mandato il Ping entro i 10-12 secondi stimati")
            .unwrap().unwrap();

        // Il client verifica che sia arrivato un Ping e risponde con un Pong (come farebbe il veicolo reale)
        assert!(msg_dal_server.is_ping(), "Il messaggio ricevuto dal server non è un Ping!");
        client_tx.send(Message::Pong(vec![])).await.unwrap();
        println!("Test: Ricevuto Ping dal server e risposto con Pong correttamente.");

        // TEST INVIO COORDINATE GPS
        // Costruisci una stringa JSON che rispecchi la tua struct 'Coordinates'
        let coords_json = r#"{"lat": 45.0708, "lon": 7.6869, "created_at": "2026-08-06 12:00:00"}"#;
        client_tx.send(Message::Text(coords_json.into())).await.unwrap();

        // Controlla se il server manda l'eco di conferma ricezione
        /*if let Some(Ok(Message::Text(confirm))) = client_rx.next().await {
            assert!(confirm.contains("Dati ricevuti"), "Il server non ha confermato le coordinate");
        }*/

        // TEST COMANDO DI STOP
        client_tx.send(Message::Text("STOP".into())).await.unwrap();

        if let Some(Ok(Message::Text(stop_confirm))) = client_rx.next().await {
            assert!(stop_confirm.contains("Tracking interrotto con successo"), "Il server non ha risposto correttamente allo STOP");
        }

        // Controlla che dopo lo STOP il server chiuda effettivamente lo stream
        /*let fine_flusso = client_rx.next().await;
        assert!(fine_flusso.is_none(), "Il server non ha chiuso la connessione dopo lo STOP");*/
    }
}


