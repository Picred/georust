pub mod database;
pub mod repository;

use database::init_db;
use repository::users_repository::UsersRepository;

use std::env::args;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let args: Vec<String> = args().collect();

    println!("Server running");

    let reset_tables = args.contains(&"--with-init".to_string());
    let pool = init_db(reset_tables).await?;

    let users_repository = UsersRepository::new(pool.clone());

    if reset_tables {
        users_repository.register_user("test", b"pswtest").await?;
    }

    let login_result = users_repository.login_user("test", b"pswtest").await;

    match login_result {
        Ok(credentials_were_correct) => {
            if credentials_were_correct {
                println!("Logged in")
            } else {
                println!("Incorrect credentials")
            }
        }
        Err(e) => println!("Something happened: {:?}", e),
    }


    Ok(())
}
