use std::collections::HashMap;  
use std::sync::Arc;
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
use super::user_session_handler::handle_user_session;

use tokio::io::{AsyncBufReadExt, BufReader};
use super::statistics::{Statistics, RequiredTimeFrame};
use super::server_messaging;


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
}

/// Structure that represents the active socket in memory
/// Fields:
/// - `socket_id` -> unique code generated at the time of connection
/// - `user_id`   -> is None before the login
/// - `socket_id` -> tx to communicate with the client (in fact it is the tx of an mpsc channel to communicate with a listening task that manages the ws_sender to transfer messages received from the mpsc channel via WebSocket)

pub struct ActiveSocket {
    pub socket_id: Uuid,
    pub user_id: Option<i64>,
    pub tx: mpsc::Sender<Message>,
}


/// This structure provides a task dispatcher and a connection handler in order to 
/// manage the server's connection with multiple clients and a server CLI. 
/// 
/// Fields:
/// - `sockets`  -> socket map to connect each ActiveSocket to a unique socket_id code (of type Uuid) so you know the connected users
/// - `state`    -> contains repository instances to communicate with the database
pub struct ConnectionManager {
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

 
    /// This method is like a task dispatcher that concurrently manages two major asynchronous execution flows:
    /// 1. Spawns a background task to continuously read, parse, and execute administrative 
    ///    commands typed directly into the server's CLI console:
    ///     - `statistics <user_id> [DAY|WEEK|MONTH]` -> Shows statistics (travel, average speed, overall movement duration, 
    ///                                                    and pause duration) for a specific user
    ///     - `send <user_id> <message>`                -> Send the text message to a specific user
    ///     - `broadcast <messsage>`                    -> Invia il messaggio testuale a tutti gli user connessi
    ///     - `help`                                    -> Shows all the commands
    /// 2. Enters an infinite loop to accept incoming TCP streams, generating a unique `Uuid`(socket_id) for each new 
    ///    connection with a user, and spawning a dedicated task that calls the `handle_connection`.
    /// 
    /// Parameters:
    /// - `self`     -> is an Arc<ConnectionManger> because it should be used by all the asynchronous task generated in this method
    /// - `listener` -> The pre-bound asynchronous `TcpListener` configured to accept incoming connections.
    pub async fn run(self: Arc<Self>, listener: TcpListener) {
        println!("Server attivo");

        // Spawn of background task to read the commands from server's CLI
        let manager_stdin = self.clone();
        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin).lines();
            
            println!("Console dei comandi attiva. Digita un comando:");

            while let Ok(Some(line)) = reader.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let command_params: Vec<&str> = trimmed.split_whitespace().collect();
                match command_params[0] { // match command type
                    "statistics" => {
                        if command_params.len() < 2 {
                            println!("Uso corretto: statistics <user_id> [DAY|WEEK|MONTH]");
                            continue;
                        }

                        let target_user_id = match command_params[1].parse::<i64>() {
                            Ok(id) => id,
                            Err(_) => {
                                println!("Errore: <user_id> deve essere un numero intero valido.");
                                continue;
                            }
                        };

                        let stats = Statistics::new(RequiredTimeFrame::CurrentMonth, &manager_stdin.state.journeys_repo);
                        if let Err(e) = stats.get_all(target_user_id).await {
                            println!("Errore durante il recupero delle statistiche: {}", e);
                        }
                    }
                    "send" => {
                        server_messaging::send(&command_params, &manager_stdin).await;
                    }
                    "broadcast" => {
                        server_messaging::broadcast(&command_params, &manager_stdin).await;
                    }
                    "help" => {
                        println!("Comandi disponibili:");
                        println!("  - statistics <user_id> [DAY|WEEK|MONTH]   -> Mostra le statistiche (tragitto, velocità media durata complessiva del movimento e durata delle pause) per uno specifico user");
                        println!("  - send <user_id> <message>                -> Invia il messaggio testuale a uno specifico user");
                        println!("  - broadcast <messsage>                    -> Invia il messaggio testuale a tutti gli user connessi");
                        println!("  - help                                    -> Mostra questo messaggio");
                    },
                    _ => {
                        println!("Comando sconosciuto. Digita 'help' per la lista dei comandi.");
                    }
                }
            }
        });

        // Loop that spawna tasks to handle each user's connection
        while let Ok((stream, _)) = listener.accept().await {   // si stabiliste la connessione TCP

            let manager = self.clone();
        
            let socket_id = Uuid::new_v4();
            println!("Nuova connessione TCP accettata. Assegnato Socket ID: {}", socket_id);

            tokio::spawn(async move {
                if let Err(e) = manager.handle_connection(stream, socket_id).await {
                    eprintln!("Errore nella gestione della connessione [{}]: {:?}", socket_id, e);
                }
            });
        }
    }


    /// Manages the initial lifecycle of a single accepted TCP connection:
    /// - performs the asynchronous WebSocket handshake
    /// - initializes the internal `mpsc` communication channel
    /// - registers the socket in the global active sockets map (initially with `user_id = None`)
    /// - spawns a dedicated micro-task for writing messages to the client
    /// - handles user's authentication (login and registration)
    /// - calls `handle_user_session` if the login is successed
    /// - at the end (when the user's session is finished) removes the socket from the global active sockets map
    ///
    /// Parameters
    /// - `stream`    -> the `TcpStream` flow of the current network connection.
    /// - `socket_id` -> the unique identifier pre-assigned to this connection.
    ///
    /// Errors
    /// - returns an error if the WebSocket protocol handshake fails.
    async fn handle_connection(&self, stream: TcpStream, socket_id: Uuid) -> Result<(), Box<dyn std::error::Error>> {

        // WebSocket handshake
        let ws_stream = accept_async(stream).await?;  

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Initialization of the internal `mpsc` communication channel between server's asynchronous tasks for this socket
        let (tx, mut rx) = mpsc::channel::<Message>(100);

        // Registers the socket in the global active sockets map with `user_id = None` because not authenticated
        {
            let active_socket = ActiveSocket {
                socket_id,
                user_id: None, 
                tx: tx.clone(),
            };
            self.sockets.write().await.insert(socket_id, active_socket);
        }

        // Spawn of a dedicated micro-task for writing messages to the client
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Authentication phase
        while let Some(result) = ws_receiver.next().await {
            let msg = result?;

            // Close management (in case the client logs out before authentication and sends a closing frame, 
            // thus interrupting the authentication phase)
            if let Message::Close(frame) = &msg {
                if let Some(cf) = frame {
                    println!("Client disconnesso durante l'auth. Codice: {}, Motivo: {}", cf.code, cf.reason);
                } else {
                    println!("Client disconnesso durante l'auth senza dettagli.");
                }
                break; 
            }

            if msg.is_text() {
                let text = msg.to_text().unwrap_or("");
                
                // If the JSON is not correct send a error message
                let auth_data: AuthRequest = match serde_json::from_str(text) {
                    Ok(data) => data,
                    Err(_) => {
                        let err_resp = AuthResponse {
                            status: "error".into(),
                            message: "JSON non valido. Invia le credenziali per accedere.".into(),
                        };
                        if let Ok(json) = serde_json::to_string(&err_resp) {
                            let _ = tx.send(Message::Text(json.into())).await;
                        }
                        continue;
                    }
                };

                match auth_data.action.as_str() { // check JSON's fiels -> "action": "..."

                    "login" => {
                        match self.state.users_repo.validate_user_credentials(&auth_data.username, auth_data.password.as_bytes()).await {

                            // Success case
                            Ok(AuthenticationStatus::Success(id)) => {
                                
                                // registers the user_id in the active sockets map
                                {
                                    let mut guard = self.sockets.write().await;
                                    if let Some(socket) = guard.get_mut(&socket_id) {
                                        socket.user_id = Some(id);
                                    }
                                }

                                // Send success message
                                let resp = AuthResponse {
                                    status: "success".into(),
                                    message: format!("Login effettuato! Benvenuto {}", auth_data.username),
                                };
                                if let Ok(json) = serde_json::to_string(&resp) {
                                    let _ = tx.send(Message::Text(json.into())).await;
                                }

                                handle_user_session(&mut ws_receiver, tx.clone(), id, &self.state).await?;

                                break;
                            }

                            // Failure case for invalid credentials
                            Ok(AuthenticationStatus::InvalidCredentials) => {
                                let resp = AuthResponse { status: "error".into(), message: "Username o password errati.".into(),};
                                if let Ok(json) = serde_json::to_string(&resp) { let _ = tx.send(Message::Text(json.into())).await; }
                            }

                            // Failure case for DB error
                            Err(e) => {
                                let resp = AuthResponse { status: "error".into(), message: format!("Errore DB: {}", e),};
                                if let Ok(json) = serde_json::to_string(&resp) { let _ = tx.send(Message::Text(json.into())).await; }
                            }
                        }
                    }

                    "register" => {
                        match self.state.users_repo.insert_user(&auth_data.username, auth_data.password.as_bytes()).await {
                            // Success case
                            Ok(_id) => {
                                let resp = AuthResponse {
                                    status: "success".into(),
                                    message: "Registrazione completata! Ora effettua il login.".into(),
                                };
                                if let Ok(json) = serde_json::to_string(&resp) {
                                    let _ = tx.send(Message::Text(json.into())).await;
                                }
                                // Here the break is NOT placed because the user has only registered, 
                                // the loop continues to allow him to "login".
                            }

                            // Failure case for DB error
                            Err(e) => {
                                let resp = AuthResponse {
                                    status: "error".into(),
                                    message: format!("Errore registrazione (es. utente esistente): {:?}", e),
                                };
                                if let Ok(json) = serde_json::to_string(&resp) {
                                    let _ = tx.send(Message::Text(json.into())).await;
                                }
                            }
                        }
                    }

                    _ => {
                        let resp = AuthResponse { status: "error".into(), message: "Usa 'login' o 'register'.".into(),};
                        if let Ok(json) = serde_json::to_string(&resp) { let _ = tx.send(Message::Text(json.into())).await; }
                    }
                }
            }
        }

        // Cleanup to disconession
        self.sockets.write().await.remove(&socket_id);
        println!("Socket [{}] rimosso dalla mappa globale causa disconnessione.", socket_id);

        Ok(())
    }
}




// test for connection manager and handle_journey_tracking

#[cfg(test)]
mod tests {

    use super::*;
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    use futures_util::{StreamExt, SinkExt};
    use sqlx::SqlitePool;
    use std::time::Duration;

    #[tokio::test]
    async fn test_user_connection_successful_flow() {
        // Initialize an isolated, temporary in-memory SQLite DB for the test
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        
        // Create the necessary tables for the test
        sqlx::query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username TEXT, password BLOB);")
            .execute(&pool).await.unwrap();
        /*sqlx::query("INSERT INTO users (id, username, password) VALUES (1, 'veicolo_test', 'password_test');")
            .execute(&pool).await.unwrap();*/
        sqlx::query("CREATE TABLE IF NOT EXISTS journeys (id INTEGER PRIMARY KEY, user_id INTEGER, lat REAL, lon REAL, is_stopped INTEGER, created_at TEXT);")
            .execute(&pool).await.unwrap();

        // Start the ConnectionManager on a random free port (port 0)
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();
        
        let manager = Arc::new(ConnectionManager::new(pool.clone()));
        let manager_clone = manager.clone();

        // Start the server in a dedicated background task
        tokio::spawn(async move {
            manager_clone.run(listener).await;
        });

        
        // CLIENT SIMULATION

        // Establish the connection with the server
        let url = format!("ws://{}", local_addr);
        let (ws_stream, _) = connect_async(&url).await.expect("Connessione fallita");
        let (mut client_tx, mut client_rx) = ws_stream.split();


        // REGISTER TEST
        let login_json = r#"{"action":"register","username":"veicolo_test","password":"password_test"}"#;
        client_tx.send(Message::Text(login_json.into())).await.unwrap();

        // Wait for the server response
        if let Some(Ok(Message::Text(response))) = client_rx.next().await {
            assert!(response.contains("success"), "La registrazione è fallita: {}", response);
            assert!(response.contains("Registrazione completata!"), "Messaggio di conferma della registrazione errato");
        } else {
            panic!("Non è stata ricevuta una risposta di testo valida per la registrazione");
        }


        // LOGIN TEST
        let login_json = r#"{"action":"login","username":"veicolo_test","password":"password_test"}"#;
        client_tx.send(Message::Text(login_json.into())).await.unwrap();

        // Wait for the server response
        if let Some(Ok(Message::Text(response))) = client_rx.next().await {
            assert!(response.contains("success"), "Il login è fallito: {}", response);
            assert!(response.contains("Login effettuato!"), "Messaggio di benvenuto errato");
        } else {
            panic!("Non è stata ricevuta una risposta di testo valida per il login");
        }

        // ACTIVE PING TEST

        // Now we test whether the server sends the active PING (set to 10 seconds in the server)
        // We use a timeout on the test to avoid hanging if the server fails
        let msg_dal_server = tokio::time::timeout(Duration::from_secs(12), client_rx.next()).await
            .expect("Il server non ha mandato il Ping entro i 10-12 secondi stimati")
            .unwrap().unwrap();

        // The client verifies that a Ping has arrived and responds with a Pong (as the actual vehicle would do)
        assert!(msg_dal_server.is_ping(), "Il messaggio ricevuto dal server non è un Ping!");
        client_tx.send(Message::Pong(vec![])).await.unwrap();
        println!("Test: Ricevuto Ping dal server e risposto con Pong correttamente.");

        // GPS COORDINATES SUBMISSION TEST
        // Construct a JSON string that matches your 'Coordinates' struct
        let coords_json = r#"{"lat": 45.0708, "lon": 7.6869, "created_at": "2026-08-06 12:00:00"}"#;
        client_tx.send(Message::Text(coords_json.into())).await.unwrap();

        let coords_json = r#"{"lat": 45.0708, "lon": 7.6869, "created_at": "2026-08-06 12:01:00"}"#;
        client_tx.send(Message::Text(coords_json.into())).await.unwrap();

        let coords_json = r#"{"lat": 45.08, "lon": 7.6869, "created_at": "2026-08-06 12:02:00"}"#;
        client_tx.send(Message::Text(coords_json.into())).await.unwrap();


        let coords_json = r#"{"lat": 45.08, "lon": 7.6869, "created_at": "2026-08-06 12:05:00"}"#;
        client_tx.send(Message::Text(coords_json.into())).await.unwrap();

        // STOP COMMAND TEST
        client_tx.send(Message::Text("STOP".into())).await.unwrap();

        if let Some(Ok(Message::Text(stop_confirm))) = client_rx.next().await {
            assert!(stop_confirm.contains("Tracking interrotto con successo"), "Il server non ha risposto correttamente allo STOP");
        }
    }
}




