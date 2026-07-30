use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordVerifier},
};
use sqlx::{Pool, Sqlite};

use crate::authenticator::Authenticator;

pub struct UsersRepository {
    pub pool: Pool<Sqlite>,
}

impl UsersRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn insert_user(&self, username: &str, password: &[u8]) -> Result<bool, sqlx::Error> {
        let password_hash = Authenticator::encrypt_password(password)?;

        let result = sqlx::query("INSERT INTO users(username, password) VALUES (?, ?);")
            .bind(username)
            .bind(password_hash)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn validate_user_credentials(
        &self,
        username: &str,
        password: &[u8],
    ) -> Result<bool, sqlx::Error> {
        let stored_password = match self.get_password_by_username(username).await {
            Ok(row_hash) => row_hash,
            Err(e) => {
                println!("[DEBUG] Errore, username non trovato: {:?}", e);
                return Ok(false);
            }
        };

        let is_valid_password = Authenticator::verify_password(password, stored_password)?;

        if is_valid_password {
            println!("[DEBUG] Password corretta")
        } else {
            println!("[DEBUG] Password errata")
        }

        Ok(is_valid_password)
    }

    async fn get_password_by_username(&self, username: &str) -> Result<String, sqlx::Error> {
        sqlx::query_scalar("SELECT password FROM users WHERE username = ?;")
            .bind(username)
            .fetch_one(&self.pool)
            .await
    }
}
