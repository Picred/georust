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
use G19::LogModule;


// Struct for communication during login/register phase
#[derive(Deserialize)]
struct AuthRequest {
    action: String, // "login" or "register"
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

    // Get user_id from socket_id
    pub async fn get_user_id_by_socket_id(&self, socket_id: &Uuid) -> Option<i64> {
        let guard = self.sockets.read().await;
        guard.get(socket_id).and_then(|socket| socket.user_id)
    }

 
    /// This method is like a task dispatcher that concurrently manages two major asynchronous execution flows:
    /// 1. Spawns a background task to continuously read, parse, and execute administrative 
    ///    commands typed directly into the server's CLI console:
    ///     - `statistics <user_id> [DAY|WEEK|MONTH]` -> Shows statistics (travel, average speed, overall movement duration, 
    ///                                                    and pause duration) for a specific user. If the third parameter is missing then it is by default DAY
    ///     - `send <user_id> <message>`                -> Sends the text message to a specific user
    ///     - `broadcast <messsage>`                    -> Sends a text message to all the active users
    ///     - `help`                                    -> Shows all the commands
    /// 2. Enters an infinite loop to accept incoming TCP streams, generating a unique `Uuid`(socket_id) for each new 
    ///    connection with a user, and spawning a dedicated task that calls the `handle_connection`.
    /// 
    /// Parameters:
    /// - `self`     -> is an Arc<ConnectionManger> because it should be used by all the asynchronous task generated in this method
    /// - `listener` -> The pre-bound asynchronous `TcpListener` configured to accept incoming connections.
    pub async fn run(self: Arc<Self>, listener: TcpListener) {
        println!("Server running");

        // Spawn of background task to read the commands from server's CLI
        let manager_stdin = self.clone();
        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin).lines();
            
            println!("Command console active. Enter a command:");

            while let Ok(Some(line)) = reader.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let command_params: Vec<&str> = trimmed.split_whitespace().collect();
                match command_params[0] { // match command type
                    "statistics" => {
                        if command_params.len() < 2 {
                            println!("Correct usage: statistics <user_id> [DAY|WEEK|MONTH]");
                            continue;
                        }

                        let target_user_id = match command_params[1].parse::<i64>() {
                            Ok(id) => id,
                            Err(_) => {
                                println!("Error: <user_id> must be a valid integer.");
                                continue;
                            }
                        };

                        let time_frame = if command_params.len() >= 3 {
                            match command_params[2].to_uppercase().as_str() {
                                "DAY" => RequiredTimeFrame::CurrentDay,
                                "WEEK" => RequiredTimeFrame::CurrentWeek,   
                                "MONTH" => RequiredTimeFrame::CurrentMonth,
                                _ => {
                                    println!("Error: Invalid timeframe. Use DAY, WEEK, or MONTH. Defaulting to DAY.");
                                    RequiredTimeFrame::CurrentDay
                                }
                            }
                        } else {
                            RequiredTimeFrame::CurrentDay
                        };

                        let stats = Statistics::new(time_frame, &manager_stdin.state.journeys_repo);
                        if let Err(e) = stats.get_all(target_user_id).await {
                            println!("Error retrieving statistics: {}", e);
                        }
                    }
                    "send" => {
                        server_messaging::send(&command_params, &manager_stdin).await;
                    }
                    "broadcast" => {
                        server_messaging::broadcast(&command_params, &manager_stdin).await;
                    }
                    "help" => {
                        println!("Available commands:");
                        println!("  - statistics <user_id> [DAY|WEEK|MONTH] -> Shows statistics (journey, average speed, total movement duration, and pause duration) for a specific user. if the third parameter is missing then it is by default DAY");
                        println!("  - send <user_id> <message> -> Sends a text message to a specific user");
                        println!("  - broadcast <message> -> Sends a text message to all connected users");
                        println!("  - help -> Shows this message");
                    },
                    _ => {
                        println!("Unknown command. Type 'help' for a list of commands.");
                    }
                }
            }
        });

        // Loop that spawns tasks to handle each user's connection
        while let Ok((stream, _)) = listener.accept().await {   // TCP connection established

            let manager = self.clone();
        
            let socket_id = Uuid::new_v4();
            G19::info!(LogModule::ConnectionManager, "connection_accept", "New TCP connection accepted. Assigned Socket ID: {}", socket_id);

            tokio::spawn(async move {
                if let Err(e) = manager.handle_connection(stream, socket_id).await {
                    G19::error!(LogModule::ConnectionManager, "connection_error", "Error handling connection [socket_id: {}]: {:?}", socket_id, e);
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
                    G19::warn!(LogModule::ConnectionManager, "connection_closing", "Client disconnected during authentication. Code: {}, Reason: {}", cf.code, cf.reason);
                } else {
                    G19::warn!(LogModule::ConnectionManager, "connection_closing", "Client disconnected during authentication without details");
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
                            message: "Invalid JSON. Please send credentials to log in.".into(),
                        };
                        if let Ok(json) = serde_json::to_string(&err_resp) {
                            let _ = tx.send(Message::Text(json.into())).await;
                        }
                        continue;
                    }
                };

                match auth_data.action.as_str() { // check JSON's fields -> "action": "..."

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
                                    message: format!("Login successful! Welcome {}", auth_data.username),
                                };
                                if let Ok(json) = serde_json::to_string(&resp) {
                                    let _ = tx.send(Message::Text(json.into())).await;
                                }

                                handle_user_session(&mut ws_receiver, tx.clone(), id, &self.state).await?;

                                break;
                            }

                            // Failure case for invalid credentials
                            Ok(AuthenticationStatus::InvalidCredentials) => {
                                let resp = AuthResponse { status: "error".into(), message: "Invalid username or password.".into(),};
                                if let Ok(json) = serde_json::to_string(&resp) { let _ = tx.send(Message::Text(json.into())).await; }
                            }

                            // Failure case for DB error
                            Err(e) => {
                                let resp = AuthResponse { status: "error".into(), message: format!("DB Error: {}", e),};
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
                                    message: "Registration completed! Please log in now.".into(),
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
                                    message: format!("Registration error (e.g., user already exists): {:?}", e),
                                };
                                if let Ok(json) = serde_json::to_string(&resp) {
                                    let _ = tx.send(Message::Text(json.into())).await;
                                }
                            }
                        }
                    }

                    _ => {
                        let resp = AuthResponse { status: "error".into(), message: "Use either 'login' or 'register'.".into(),};
                        if let Ok(json) = serde_json::to_string(&resp) { let _ = tx.send(Message::Text(json.into())).await; }
                    }
                }
            }

        }

        // Cleanup to disconession
        self.sockets.write().await.remove(&socket_id);
        G19::info!(LogModule::ConnectionManager, "connection_closing", "Connection closed: removed socket [{}] from active sockets map", socket_id);

        Ok(())
    }
}
