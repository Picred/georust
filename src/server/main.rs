// DEVELOPMENT
// -----------------------
// Non cancellabile
pub mod authenticator;
pub mod database;
pub mod models;
pub mod repository;
// -----------------------
pub mod connection_manager;
pub mod journey_tracking;

use std::env::args;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let _args: Vec<String> = args().collect();

    println!("Server running");

    Ok(())
}

