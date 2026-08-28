use G19::LogModule;
use sqlx::{Pool, Sqlite, Row};

use crate::authenticator::Authenticator;

/// Represents the outcome of a user authentication attempt.
/// 
/// # Variants:
/// - `Succsss(i64)`: Authentication succeeded. Contains the unique `user_id` of the user.
/// - `InvalidCredentials`: Authentication failed due to invalid credentials (user not found or mismatched password).
#[derive(Debug, PartialEq, Eq)]
pub enum AuthenticationStatus {
    Success(i64),
    InvalidCredentials,
}

/// Repository responsible for user persistence and credential validation in SQLite.
pub struct UsersRepository {
    pub pool: Pool<Sqlite>,
}

impl UsersRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }


    /// Hashes the provided password and stores a new user record in the database.
    ///
    /// Arguments:
    ///
    /// - `username` - The unique username for the new account.
    /// - `password` - The raw password provided as a byte slice (`&[u8]`).
    ///
    /// Returns the database-generated ID (`last_insert_rowid`) of the inserted user,
    /// or a [`sqlx::Error`] if the operation fails (e.g., uniqueness constraint violation).
    pub async fn insert_user(&self, username: &str, password: &[u8]) -> Result<i64, sqlx::Error> {
        let password_hash = Authenticator::encrypt_password(password)?;

        let result = sqlx::query("INSERT INTO users(username, password) VALUES (?, ?);")
            .bind(username)
            .bind(password_hash)
            .execute(&self.pool)
            .await?;

        G19::info!(LogModule::Authenticator, "registration_attempt", "{:?} Successfully registered!", username);
        Ok(result.last_insert_rowid())
    }


    /// Verifies user credentials against the stored password hash.
    ///
    /// 1. Queries the database for the user record via [`get_password_and_id_by_username`](Self::get_password_and_id_by_username).
    /// 2. If the user is missing ([`sqlx::Error::RowNotFound`]), returns [`AuthenticationStatus::InvalidCredentials`] instead of raising a database error.
    /// 3. Validates the raw password against the stored hash using [`Authenticator::verify_password`].
    pub async fn validate_user_credentials(
        &self,
        username: &str,
        password: &[u8],
    ) -> Result<AuthenticationStatus, sqlx::Error> {
        let (user_id, stored_password) = match self.get_password_and_id_by_username(username).await {
            Ok(row_hash) => row_hash,
            Err(sqlx::Error::RowNotFound) =>    {
                G19::warn!(LogModule::Authenticator, "login_attempt", "{:?} Failed to log in!", username);
                return Ok(AuthenticationStatus::InvalidCredentials);
            },
            Err(e) => return Err(e)
        };


        if Authenticator::verify_password(password, stored_password) {
            G19::info!(LogModule::Authenticator, "login_success", "{:?} Successfully logged in!", username);
            Ok(AuthenticationStatus::Success(user_id))
        } else {
            G19::warn!(LogModule::Authenticator, "login_attempt", "{:?} Failed to log in!", username);
            Ok(AuthenticationStatus::InvalidCredentials)
        }
    }


    /// Fetches the user ID and hashed password for a given username.
    /// 
    /// # Errors
    /// Returns [`sqlx::Error::RowNotFound`] if no user matching `username` exists.
    async fn get_password_and_id_by_username(&self, username: &str) -> Result<(i64, String), sqlx::Error> {
        let sql = "SELECT id, password FROM users WHERE username = ?;";
        let row = sqlx::query(sql).bind(username).fetch_one(&self.pool).await?;

        let user_id: i64 = row.get(0);
        let stored_password: String = row.get(1);


        Ok((user_id, stored_password))
    }
}







#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connection error with RAM db ");

        sqlx::query(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL
            );",
        )
        .execute(&pool)
        .await
        .expect("Cannot create users table");

        pool
    }

    #[tokio::test]
    async fn test_insert_user_success() {
        let pool = setup_db().await;
        let repo = UsersRepository::new(pool);

        let user_id = repo.insert_user("andrei", b"password123").await;
        assert!(user_id.is_ok());
        assert_eq!(user_id.unwrap(), 1); // Primo utente inserito deve avere ID 1
    }

    #[tokio::test]
    async fn test_validate_credentials_correct_password() {
        let pool = setup_db().await;
        let repo = UsersRepository::new(pool);

        let inserted_id = repo
            .insert_user("andrei", b"password123")
            .await
            .unwrap();

        let status = repo
            .validate_user_credentials("andrei", b"password123")
            .await
            .unwrap();

        assert_eq!(status, AuthenticationStatus::Success(inserted_id));
    }

    #[tokio::test]
    async fn test_validate_credentials_wrong_password() {
        let pool = setup_db().await;
        let repo = UsersRepository::new(pool);

        repo.insert_user("andrei", b"password123").await.unwrap();

        let status = repo
            .validate_user_credentials("andrei", b"wrong_password")
            .await
            .unwrap();

        assert_eq!(status, AuthenticationStatus::InvalidCredentials);
    }

    #[tokio::test]
    async fn test_validate_credentials_user_not_found() {
        let pool = setup_db().await;
        let repo = UsersRepository::new(pool);

        let status = repo
            .validate_user_credentials("non_esisto", b"password123")
            .await
            .unwrap();

        assert_eq!(status, AuthenticationStatus::InvalidCredentials);
    }
}