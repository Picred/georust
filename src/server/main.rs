pub mod database;
pub mod repository;

use database::init_db;
use repository::auth_repository::AuthRepository;

use std::env::args;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let args: Vec<String> = args().collect();

    println!("Server running");

    let reset_tables = args.contains(&"--with-init".to_string());

    let pool = init_db(reset_tables).await?;


    let auth_repository = AuthRepository::new(pool.clone());


    auth_repository.create("test", "pswtest").await?;

    
    Ok(())
}
