use sqlx::{Pool, Sqlite, Row};

use crate::authenticator::Authenticator;

#[derive(Debug, PartialEq, Eq)]
pub enum AuthenticationStatus {
    Success(i64),
    InvalidCredentials,
}

pub struct UsersRepository {
    pub pool: Pool<Sqlite>,
}

impl UsersRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn insert_user(&self, username: &str, password: &[u8]) -> Result<i64, sqlx::Error> {
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
        let (user_id, stored_password) = match self.get_password_and_id_by_username(username).await {
            Ok(row_hash) => row_hash,
            Err(sqlx::Error::RowNotFound) => return Ok(AuthenticationStatus::InvalidCredentials),
            Err(e) => return Err(e)
        };

        println!("[DEBUG] id: {}, stored_password: {}", user_id, stored_password);

        if Authenticator::verify_password(password, stored_password) {
            Ok(AuthenticationStatus::Success(user_id))
        } else {
            Ok(AuthenticationStatus::InvalidCredentials)
        }
    }

    async fn get_password_and_id_by_username(&self, username: &str) -> Result<(i64, String), sqlx::Error> {
        let sql = "SELECT id, password FROM users WHERE username = ?;";
        let row = sqlx::query(sql).bind(username).fetch_one(&self.pool).await?;

        let user_id: i64 = row.get(0);
        let stored_password: String = row.get(1);


        Ok((user_id, stored_password))
        // sqlx::query_scalar("SELECT id, password FROM users WHERE username = ?;")
        //     .bind(username)
        //     .fetch_one(&self.pool)
        //     .await
    }
}
