pub mod database;
pub mod repository;

use database::init_db;

use std::env::args;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let args: Vec<String> = args().collect();

    println!("Server running");

    let reset_tables = args.contains(&"--with-init".to_string());
    

    let _db = tokio::spawn(init_db(reset_tables)).await.unwrap()?;

    Ok(())
}
