use crate::database::DatabasePool;

pub struct AuthRepository {
    pub pool: DatabasePool,
}

impl AuthRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, username: &str, password: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("INSERT INTO vehicles(username, password) VALUES (?,?);")
            .bind(username)
            .bind(password)
            .execute(&self.pool.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
