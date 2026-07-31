pub mod authenticator;
pub mod database;
pub mod repository;
pub mod models;

use database::init_db;
use std::env::args;

use repository::journeys_repository::JourneysRepository;


#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let args: Vec<String> = args().collect();

    println!("Server running");

    let reset_tables = args.contains(&"--with-init".to_string());
    let pool = init_db(reset_tables).await?;

    
    let journeys_repository = JourneysRepository::new(pool.clone());

    journeys_repository.insert_journey(2, 16.2, 23.6, 51.0).await?;
    let journeys = journeys_repository.get_full_journey_by_user_id(2).await?;

    for journey in journeys {
        println!("all journeys of user 2: {:?}", journey);
    }
    
    Ok(())
}
