use sqlx::{Pool, Sqlite};

use crate::models::journey::Journey;

pub struct JourneysRepository {
    pub pool: Pool<Sqlite>,
}

impl JourneysRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn insert_journey(
        &self,
        user_id: i32,
        lat: f64,
        lon: f64,
    ) -> Result<(), sqlx::Error> {
        let sql = "INSERT INTO journeys(user_id, lat, lon) VALUES (?, ?, ?);";
        sqlx::query(sql)
            .bind(user_id)
            .bind(lat)
            .bind(lon)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_full_journey_by_user_id(
        &self,
        user_id: i32,
    ) -> Result<Vec<Journey>, sqlx::Error> {
        let sql = "SELECT * FROM journeys WHERE user_id = ?;";
        let journeys: Vec<Journey> = sqlx::query_as(sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(journeys)
    }
}
