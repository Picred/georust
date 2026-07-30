use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use sqlx::{Pool, Row, Sqlite, sqlite::SqliteRow};

pub struct UsersRepository {
    pub pool: Pool<Sqlite>,
}

impl UsersRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn register_user(
        &self,
        username: &str,
        password: &[u8],
    ) -> Result<bool, sqlx::Error> {
        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password, &salt)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?
            .to_string();

        let result = sqlx::query("INSERT INTO users(username, password) VALUES (?, ?);")
            .bind(username)
            .bind(password_hash)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn login_user(&self, username: &str, password: &[u8]) -> Result<bool, sqlx::Error> {
        // 1. Recuperiamo l'hash salvato nel DB
        let stored_hash_str = match self.get_password_by_username(username).await {
            Ok(hash) => hash,
            Err(e) => {
                println!("[DEBUG] Errore, username non trovato: {:?}", e);
                return Ok(false);
            }
        };

        let parsed_hash = match PasswordHash::new(&stored_hash_str) {
            Ok(hash) => hash,
            Err(_) => return Ok(false), // Hash nel DB corrottio non valido
        };

        let argon2 = Argon2::default();
        let is_valid = argon2.verify_password(password, &parsed_hash).is_ok();

        if is_valid{
            println!("[DEBUG] Password corretta")
        }
        else{
            println!("[DEBUG] Password errata")
        }

        Ok(is_valid)
    }

    async fn get_password_by_username(&self, username: &str) -> Result<String, sqlx::Error> {
        sqlx::query_scalar("SELECT password FROM users WHERE username = ?;")
            .bind(username)
            .fetch_one(&self.pool)
            .await
    }
}
