// DEVELOPMENT
// -----------------------
// Non cancellabile
pub mod authenticator;
pub mod database;
pub mod models;
pub mod repository;
pub mod statistics;
pub mod connection_manager;
pub mod user_session_handler;
pub mod user_state_handler;
pub mod utils;
pub mod server_messaging;
// -----------------------

use database::init_db;
use std::{error::Error, sync::Arc};
use tokio::net::TcpListener;
use sqlx::{Pool, Sqlite};

use crate::{connection_manager::ConnectionManager};

use G19::utils::logger::{Logger, LogLevel, LogModule};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    Logger::init("logs/server.log", LogLevel::Debug, Duration::from_secs(3))
        .await
        .expect("failed to init logger");

    G19::info!(LogModule::Main, "startup", "Georust server starting up");
    

    let pool:Pool<Sqlite> = init_db(true).await?;
    let manager = Arc::new(ConnectionManager::new(pool));
    let addr = "127.0.0.1:9001".to_string();
    let listener = TcpListener::bind(&addr).await?;
    manager.run(listener).await;

    Logger::flush().await;

    Ok(())
}

