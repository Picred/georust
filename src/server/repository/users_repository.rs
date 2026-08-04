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
    }
}







#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    // db in ram
    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("errore di connessione al db in ram");

        sqlx::query(
            "CREATE TABLE users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL
            );",
        )
        .execute(&pool)
        .await
        .expect("Impossibile creare la tabella users");

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