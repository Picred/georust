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
            - scelta della modalità:
                - TRACKING:
                    - invio coordinate
                    - aggiornamento dello status dell'user
                - STATISTICS (menù per scegliere quale statistica si vuole ottenere)
        - disconnessione
 */


use std::collections::HashMap;  
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use serde::{Deserialize, Serialize};
use crate::users_repository::{UsersRepository, AuthenticationStatus};

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
    user_id: Option<i32>,
}

// Struct temporanea per il parsing del JSON della modalità
#[derive(Deserialize)]
struct ModeRequest {
    mode: String, // Ci aspettiamo "tracking" o "statistics"
}

enum Mode {
    Tracking,
    Statistics,
}

// La struttura che rappresenta il socket in memoria
pub struct ActiveSocket {
    pub socket_id: Uuid,         // Il codice univoco generato al momento della connessione
    pub user_id: Option<i32>,     // è None prima del login
    pub tx: mpsc::Sender<Message>, // tx da clonare in un task per trasmettere al ws_sender (gestito da un task in ascolto su rx) per poi comunicare con il client
}

pub struct ConnectionManager {
    // mappa dei socket per collegare ogni ActiveSocket a un codice univoco socket_id (di tipo Uuid)
    pub sockets: Arc<RwLock<HashMap<Uuid, ActiveSocket>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            sockets: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ottenere user_id partendo da socked_id
    pub async fn get_user_id_by_socket_id(&self, socket_id: &Uuid) -> Option<i32> {
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
        let mut autenticato_user_id: Option<i32> = None;
        let mut autenticato_user_id: Option<i32> = None;

        // ==========================================================
        // FASE DI AUTENTICAZIONE (Gira finché l'utente non è loggato)
        // ==========================================================
        while let Some(result) = ws_receiver.next().await {
            let msg = result?;
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
                        match self.users_repo.validate_user_credentials(&auth_data.username, auth_data.password.as_bytes()).await {

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
                        match self.users_repo.insert_user(&auth_data.username, auth_data.password.as_bytes()).await {

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

        // Controllo di sicurezza: se il client chiude la connessione prima di loggarsi con successo,
        // si esce anticipatamente eseguendo il cleanup.
        let user_id = match autenticato_user_id {
            Some(id) => id,
            None => {
                self.sockets.write().await.remove(&socket_id);
                return Ok(());
            }
        };


        // ==========================================================
        // FASE DI ATTESA SELEZIONE MODALITÀ (Tramite JSON)
        // ==========================================================
        let mut chosen_mode: Optione<Mode> = None;

        while let Some(result) = ws_receiver.next().await {
            let msg = result?;
            if msg.is_text() {
                let text = msg.to_text().unwrap_or("");
                
                // Tentativo di parsing del JSON: {"mode": "..."}
                if let Ok(req) = serde_json::from_str::<ModeRequest>(text) {
                    match req.mode.to_lowercase().as_str() {
                        "tracking" => {
                            chosen_mode = Some(Mode::Tracking);
                            break; // Esce dal loop di selezione
                        }
                        "statistics" => {
                            chosen_mode = Some(Mode::Statistics);
                            break; // Esce dal loop di selezione
                        }
                        _ => {
                            let _ = tx.send(Message::Text(
                                "{\"error\":\"Modalità non valida. Usa 'tracking' o 'statistics'\"}".into()
                            )).await;
                        }
                    }
                } else {
                    let _ = tx.send(Message::Text(
                        "{\"error\":\"Formato non valido. Invia un JSON tipo: {\\\"mode\\\": \\\"tracking\\\"}\"}".into()
                    )).await;
                }
            }
        }

        // ==========================================================
        // FASE DI DISPATCHING DELLA MODALITÀ (Viene creato un task che gestice esecuzione della modalità scelta)
        // ==========================================================
        match chosen_mode {
            Some(Mode::Tracking) => {
                let _ = tx.send(Message::Text("{\"status\":\"success\",\"message\":\"Modalità Tracking avviata. Invia coordinate o 'STOP'\"}".into())).await;
                // AVVIO del task per il tracking (per ricezione ed inserimento delle coordinate nel db)
                // TODO
            }
            Some(Mode::Statistics) => {
                let _ = tx.send(Message::Text("{\"status\":\"success\",\"message\":\"Modalità Statistiche avviata.\"}".into())).await;
                // AVVIO del task statistics (per ottenere le statistiche)
                // TODO
            }
            None => {}
        }

        // CLEANUP AL DISCONNECT
        self.sockets.write().await.remove(&socket_id);
        println!("Socket [{}] rimosso dalla mappa globale causa disconnessione.", socket_id);

        Ok(())
    }
}



