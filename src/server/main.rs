pub mod authenticator;
pub mod database;
pub mod repository;

use database::init_db;
use repository::users_repository::UsersRepository;

use std::env::args;

use crate::repository::users_repository::AuthenticationStatus;

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

    match users_repository.validate_user_credentials("test", b"pswtest").await{
        Ok(AuthenticationStatus::Success) => println!("Logged in!"),
        Ok(AuthenticationStatus::InvalidCredentials) => println!("Incorrect credentials"),
        Err(e) => println!("Database error: {:?}", e),
    }

    Ok(())
}
