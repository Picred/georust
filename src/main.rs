use std::sync::Arc;

mod client;
mod server;

use client::config::Config;
use client::coord_gen::CoordGenerator;
use client::network::ClientNetwork;

use server::database;
use server::network::listener::ServerListener;
use server::repository::{
    journeys_repository::JourneysRepository,
    users_repository::UsersRepository,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_addr = "127.0.0.1:8080";

    let pool = database::init_db(false).await?;
    let users_repo = Arc::new(UsersRepository::new(pool.clone()));
    let journeys_repo = Arc::new(JourneysRepository::new(pool.clone()));

    let listener = ServerListener::new(users_repo, journeys_repo);

    tokio::spawn(async move {
        if let Err(e) = listener.listen(server_addr).await {
            eprintln!("[SERVER ERROR]: {}", e);
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let cfg = Config::load("./config/client_config.txt")?;
    let mut generator = CoordGenerator::init(&cfg.coord_file_path)?;

    ClientNetwork::run_client(server_addr, &cfg, &mut generator).await?;

    Ok(())
}