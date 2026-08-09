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


### Example usage of Statistics
journeys table:

| id | user_id | lat | lon | created_at |
| :--- | :--- | :--- | :--- | :--- |
| 1 | 1 | 36.74271559322131 | 14.7532649702098 | 2026-08-07 12:50:00 |
| 2 | 1 | 37.639413644878196 | 14.892653008838272 | 2026-08-07 13:50:00 |
| 10 | 1 | 36.74381604442879 | 14.787597244376057 | 2026-08-07 14:50:00 |

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
    let total_hours = stats.get_full_journeys_duration_by_user_id(1).await?; // 2.0

    Ok(())
}

```