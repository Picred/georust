pub mod database;
pub mod repository;

use database::init_db;

use std::env::args;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let mut _args: Vec<String> = args().skip(1).collect();

    println!("Server avviato");

    let reset_tables = true;
    let _db = tokio::spawn(init_db(reset_tables)).await.unwrap()?;

    Ok(())
}
