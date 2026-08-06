// DEVELOPMENT
// -----------------------
// Non cancellabile
pub mod authenticator;
pub mod database;
pub mod models;
pub mod repository;
pub mod statistics;
// -----------------------
pub mod connection_manager;
pub mod journey_tracking;

use database::init_db;
use std::sync::Arc;
use tokio::net::TcpListener;
use sqlx::{Pool, Sqlite};

use crate::{connection_manager::ConnectionManager};

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {

    let pool:Pool<Sqlite> = init_db(false).await?;
    let manager = Arc::new(ConnectionManager::new(pool));
    let addr = "127.0.0.1:9001".to_string();
    let listener = TcpListener::bind(&addr).await?;
    manager.run(listener).await;

    Ok(())
}

