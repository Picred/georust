use sqlx::{Pool, Row, Sqlite, sqlite::SqliteRow};
use tokio::sync::Mutex;

pub struct JourneysRepository {
    pub pool: Pool<Sqlite>,
    pub journeys_num: Mutex<i32>
}

impl JourneysRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self {
            pool,
            journey_num: Mutex::new(0),
         }
    }

    // restituisce l'id del nuovo journey
    pub async fn new_journey(&self) -> i32 {
        let mut guard = self.journey_num.lock().await;
        *guard += 1;
        *guard
    }


    pub async fn insert_journey_waypoint(
        &self,
        journey_id: i32,
        user_id: i32,
        lat: f64,
        lon: f64,
        pos_time: String // che deve essere in formato "YYYY-MM-DD HH:MM:SS"
    ) -> Result<(), sqlx::Error> {
        let sql = "INSERT INTO journeys(id, user_id, lat, lon, pos_time) VALUES (?, ?, ?, ?, ?);";
        sqlx::query(sql)
            .bind(journey_id)
            .bind(user_id)
            .bind(lat)
            .bind(lon)
            .bind(pos_time) // se il formato è giusto, la conversione da String a DATETIME è automatica
            .execute(&self.pool)
            .await?;
        Ok(())
    }


    // restuisce un journey (composto JourneyWaypoints in ordine di tempo crescente) 
    pub async fn get_full_journey_by_journey_id(
        &self,
        journey_id: i32,
    ) -> Result<Vec<JourneyWaypoints>, sqlx::Error> {
        let sql = "SELECT * FROM journeys WHERE id = ? ORDER BY pos_time ASC;";
        let journey: Vec<JourneyWaypoints> = sqlx::query_as(sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(journey)
    }
}