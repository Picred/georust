use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use sqlx::{Pool, Sqlite, Row};

pub struct VehiclesRepository {
    pub pool: Pool<Sqlite>,
}

impl VehiclesRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn register_vehicle(
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

        let result =
            sqlx::query("INSERT INTO vehicles(username, password, salt) VALUES (?, ?, ?);")
                .bind(username)
                .bind(password_hash)
                .bind(salt.to_string())
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn login_vehicle(
        &self,
        username: &str,
        password: &[u8],
    ) -> Result<bool, sqlx::Error> {

        todo!();
        let row_pass = sqlx::query("SELECT password FROM vehicles WHERE username = ?;")
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;

        println!("res pass: {:?}", row_pass);

        Ok(true)
        
    }
}
