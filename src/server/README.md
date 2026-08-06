# Server

## Debug mode
- Normal compiling: `cargo run --bin server`
- Initialize again the database: `cargo run --bin server -- --with-init`


### Example usage of UserRepository

```rust
// src/server/main.rs

use std::env::args;
use database::init_db;
use repository::users_repository::{UsersRepository, AuthenticationStatus};

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let args: Vec<String> = args().collect();

    println!("Server running");

    let reset_tables = args.contains(&"--with-init".to_string());
    let pool = init_db(reset_tables).await?;

    let users_repository = UsersRepository::new(pool.clone());

    if reset_tables {
        let _inserted_user_id = users_repository.insert_user("test", b"pswtest").await?;
    }

    match users_repository.validate_user_credentials("test", b"pswtest").await{
        Ok(AuthenticationStatus::Success(user_id)) => println!("Logged in! user = {}", user_id ),
        Ok(AuthenticationStatus::InvalidCredentials) => println!("Incorrect credentials"),
        Err(e) => println!("Database error: {:?}", e),
    }
    
    Ok(())
}
```



### Example usage of JourneyRepository
```rust
// src/server/main.rs

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

    journeys_repository.insert_journey(1, 16.2, 23.6, 51.0).await?;
    let journeys = journeys_repository.get_full_journey_by_user_id(1).await?;

    for journey in journeys{
        println!("all journeys of user 1: {:?}", journey);
    }

    let journey = journeys_repository.get_journey_by_id(1).await?;
    println!("Journey: {:?}", journey);
    Ok(())
}
```

### Example usage of Statistics
```rust
use crate::statistics::{Statistics, RequiredTimeFrame};

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let _args: Vec<String> = args().collect();

    let stats = Statistics::new(RequiredTimeFrame::CurrentDay);
    // let mut stats = Statistics::new(RequiredTimeFrame::CurrentWeek);
    // let mut stats = Statistics::new(RequiredTimeFrame::CurrentMonth);

    // println!("Initial timeframe {:?}", stats.get_timeframe());
    // stats.set_timeframe(RequiredTimeFrame::CurrentWeek);
    // println!("Updated timeframe {:?}", stats.get_timeframe());


    let range = stats.convert_timeframe_to_range();

    println!("{:?}", range);
    Ok(())
}
```