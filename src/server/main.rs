pub mod database;
pub mod repository;

use database::init_db;
use repository::vehicles_repository::VehiclesRepository;

use std::env::args;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let args: Vec<String> = args().collect();

    println!("Server running");

    let reset_tables = args.contains(&"--with-init".to_string());

    let pool = init_db(reset_tables).await?;


    let vehicles_repository = VehiclesRepository::new(pool.clone());


    // vehicles_repository.register_vehicle("test", b"pswtest").await?;
    vehicles_repository.login_vehicle("test2", b"pswtest").await?;

    
    Ok(())
}
