pub mod authenticator;
pub mod database;
pub mod models;
pub mod repository;

use database::init_db;
use std::env::args;

use repository::journeys_repository::JourneysRepository;

use crate::repository::users_repository::UsersRepository;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let args: Vec<String> = args().collect();

    println!("Server running");

    let reset_tables = args.contains(&"--with-init".to_string());
    let pool = init_db(reset_tables).await?;
    let users_repository = UsersRepository::new(pool.clone());
    if reset_tables {
        users_repository.insert_user("test", b"pswtest").await?;
    }

    let journeys_repository = JourneysRepository::new(pool.clone());

    let journey = journeys_repository.get_journey_by_id(2).await?;

    println!("Journey: {:?}", journey);

    Ok(())
}
