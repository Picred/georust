# Server

## Debug mode
- Normal compiling: `cargo run --bin server`
- Initialize again the database: `cargo run --bin server -- --with-init`

### Example of `main.rs`

Run: `cargo run --bin server [-- [--reset] [--statistics]]`
Use: `--reset` to reset database table to zero, `--statistics` to run server in that specific mode with an interactive CLI.

```rust
// DEVELOPMENT
// -----------------------
pub mod authenticator;
pub mod connection_manager;
pub mod database;
pub mod journey_tracking;
pub mod models;
pub mod repository;
pub mod statistics;
pub mod user_state_handler;
pub mod utils;
// -----------------------


use database::init_db;
use std::{env::args, error::Error, sync::Arc};
use tokio::net::TcpListener;

use crate::{
    connection_manager::ConnectionManager,
    repository::journeys_repository::JourneysRepository,
    statistics::{RequiredTimeFrame, Statistics},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().collect();

    println!("[INFO]: Server running");

    let statistics_flag = args.contains(&"--statistics".to_string());
    let reset_tables_flag = args.contains(&"--reset".to_string());

    // CONFIGURAZIONE
    let pool= init_db(reset_tables_flag).await?;
    
    // TODO
    // let logger = Logger::new("lorem.ipsum"); 
    let journeys_repository = JourneysRepository::new(pool.clone());

    let statistics = Statistics::new(RequiredTimeFrame::CurrentDay, journeys_repository);

    let task_dispatcher = Arc::new(ConnectionManager::new(pool.clone()));
    let addr = "127.0.0.1:9001".to_string();
    let listener = TcpListener::bind(&addr).await?;

    // -------------------------- TASK LOGGER --------------------------
    let logger_handle = tokio::spawn(logger.start_logging());
    
    // -------------------------- TASK DISPATCHER --------------------------
    if !statistics_flag {
        let task_dispatcher_handle = tokio::spawn(task_dispatcher.run(listener));
        tokio::join!(task_dispatcher_handle);
    
    // -------------------------- TASK STATISTICHE --------------------------
    } else {
        let statistics_handler = tokio::spawn(statistics.run());
        tokio::join!(statistics_handler);
    }

    tokio::join!(logger_handle);
    Ok(())
}

```

### Usage example of UserRepository

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



### Usage example of JourneyRepository
```rust
// src/server/main.rs

use std::env::args;
use repository::journeys_repository::JourneysRepository;
use database::init_db;
use repository::users_repository::UsersRepository;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let _args: Vec<String> = args().collect();

    println!("Server running");

    let reset_tables = _args.contains(&"--with-init".to_string());
    let pool = init_db(reset_tables).await?;

    let users_repository = UsersRepository::new(pool.clone());
    let journeys_repository = JourneysRepository::new(pool.clone());

    if reset_tables {
        let inserted_user_id = users_repository.insert_user("test", b"pswtest").await?;

        journeys_repository.insert_journey_waypoint(inserted_user_id, 45.4642, 9.1900, "2026-08-05 12:00:00".to_string()).await?;
        journeys_repository.insert_journey_waypoint(inserted_user_id, 45.4650, 9.1950, "2026-08-05 12:15:00".to_string()).await?;
    
        let journey= journeys_repository.get_full_journey_by_user_id(inserted_user_id).await?;
    
        for journey_waypoint in journey{
            println!("all journey waipoints of user {}: {:?}", inserted_user_id, journey_waypoint);
        }
    }

    Ok(())
}
```


### Usage example of Statistics
journeys table:

| id | user_id | lat | lon | is_stopped | created_at |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1 | 1 | 36.74271559322131 | 14.7532649702098 | 0 | 2026-08-07 12:50:00 |
| 2 | 1 | 37.639413644878196 | 14.892653008838272 | 0 | 2026-08-07 13:50:00 |
| 10 | 1 | 36.74381604442879 | 14.787597244376057 | 0 | 2026-08-07 14:50:00 |

> Distance between id `1` and id `10` (*Google Maps*) =~ 200km

```rust
use crate::{
    database::init_db,
    repository::journeys_repository::JourneysRepository,
    statistics::{RequiredTimeFrame, Statistics},
};

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = init_db(false).await?;

    let journeys_repository = JourneysRepository::new(pool.clone());
    let stats = Statistics::new(RequiredTimeFrame::CurrentDay, journeys_repository);

    let total_distance_km = stats.get_traveled_distance_by_user_id(1).await?; // ~200 
    let average_speed = stats.get_average_speed_by_user_id(1).await?; // ~100 km/h
    let total_journeys_hours = stats.get_full_journeys_duration_by_user_id(1).await?; // 2.0
    
    let total_pauses_hours = stats.get_pauses_hours_by_user_id(1).await?;

    Ok(())
}

```