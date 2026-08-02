use sqlx::{Pool, Sqlite};

use crate::authenticator::Authenticator;

#[derive(Debug, PartialEq, Eq)]
pub enum AuthenticationStatus {
    Success(i32),
    InvalidCredentials,
}

pub struct UsersRepository {
    pub pool: Pool<Sqlite>,
}

impl UsersRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn insert_user(&self, username: &str, password: &[u8]) -> Result<i32, sqlx::Error> {
        let password_hash = Authenticator::encrypt_password(password)?;

        let result = sqlx::query("INSERT INTO users(username, password) VALUES (?, ?);")
            .bind(username)
            .bind(password_hash)
            .execute(&self.pool)
            .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn validate_user_credentials(
        &self,
        username: &str,
        password: &[u8],
    ) -> Result<AuthenticationStatus, sqlx::Error> {
        let (user_id, stored_password) = match self.get_user_data_by_username(username).await {
            Ok(data) => data,
            Err(sqlx::Error::RowNotFound) => return Ok(AuthenticationStatus::InvalidCredentials),
            Err(e) => return Err(e), // Se il DB cade, lanciamo l'errore vero e proprio!
        };

        if Authenticator::verify_password(password, stored_password) {
            Ok(AuthenticationStatus::Success(user_id))
        } else {
            Ok(AuthenticationStatus::InvalidCredentials)
        }
    }


    // restituisce un result con tupla che contiene user_id e password criptata contenuta nel db relativa a quel user
    async fn get_user_data_by_username(&self, username: &str) -> Result<(i32, String), sqlx::Error> {
        sqlx::query_as::<(i32, String), _>("SELECT id, password FROM users WHERE username = ?;")
            .bind(username)
            .fetch_one(&self.pool)
            .await
    }
}