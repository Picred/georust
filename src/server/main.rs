pub mod authenticator;
pub mod database;
pub mod utils;
pub mod repository;
pub mod connection_manager;

use database::init_db;
use std::env::args;
use std::sync::Arc;
use tokio::net::TcpListener;
use connection_manager::ConnectionManager;
use utils::ServerState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // inizializzo il db
    let pool:Pool<Sqlite> = init_db(false).await?;

    // Creazione del ConnectionManager
    let manager = Arc::new(ConnectionManager::new(pool));

    // Legge l'indirizzo da una variabile d'ambiente, altrimenti usa 0.0.0.0:8080 di default
    let addr = std::env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    // Avvio del server TCP
    let listener = TcpListener::bind(&addr).await?;

    // Avvio del Task Dispatcher
    manager.run(listener, ServerState).await;

    Ok(())
}