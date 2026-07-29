use sqlx::{Pool, Sqlite};


pub struct AuthRepository {
    pub pool: Pool<Sqlite>,
}

impl AuthRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, username: &str, password: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("INSERT INTO vehicles(username, password) VALUES (?,?);")
            .bind(username)
            .bind(password)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
