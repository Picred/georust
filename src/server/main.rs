pub mod authenticator;
pub mod connection_manager;
pub mod database;
pub mod models;
pub mod repository;
pub mod server_messaging;
pub mod statistics;
pub mod user_session_handler;
pub mod user_state_handler;
pub mod utils;

use database::init_db;
use sqlx::{Pool, Sqlite};
use std::{env::args, error::Error, sync::Arc};
use tokio::net::TcpListener;

use crate::connection_manager::ConnectionManager;

use georust::utils::logger::{LogLevel, LogModule, Logger};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let reset_tables_flag = args()
        .collect::<Vec<String>>()
        .contains(&"--with-init".to_string());

    Logger::init("logs/server.log", LogLevel::Debug, Duration::from_secs(120))
        .await
        .expect("failed to init logger");

    georust::info!(LogModule::Main, "startup", "Georust server starting up");

    let pool: Pool<Sqlite> = init_db(reset_tables_flag).await?;
    let manager = Arc::new(ConnectionManager::new(pool));
    let addr = "127.0.0.1:9001".to_string();
    let listener = TcpListener::bind(&addr).await?;
    manager.run(listener).await;

    Logger::flush().await;

    Ok(())
}
