use std::str::FromStr;

use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};


pub async fn init_db(reset_tables: bool) -> Result<Pool<Sqlite>, sqlx::Error> {
    let options = SqliteConnectOptions::from_str("sqlite://src/server/database/database.sqlite")?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await?;

    if reset_tables {
        sqlx::query("DROP TABLE IF EXISTS users")
            .execute(&pool)
            .await?;
        sqlx::query("DROP TABLE IF EXISTS journeys")
            .execute(&pool)
            .await?;
        println!("[INFO] Database reset");
    }

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      username TEXT UNIQUE NOT NULL,
      password TEXT NOT NULL
      );",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS journeys (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      user_id INTEGER NOT NULL,
      lat REAL NOT NULL,
      lon REAL NOT NULL,
      is_stopped INTEGER NOT NULL,
      created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

      FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
      );",
    )
    .execute(&pool)
    .await?;


    Ok(pool)
}
