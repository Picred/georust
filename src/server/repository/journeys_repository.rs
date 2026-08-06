use sqlx::{Pool, Sqlite};
use crate::models::journey_waypoint::JourneyWaypoint;

pub struct JourneysRepository {
    pub pool: Pool<Sqlite>,
}

impl JourneysRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn insert_journey_waypoint(
        &self,
        user_id: i64,
        lat: f64,
        lon: f64,
        created_at: String
    ) -> Result<(), sqlx::Error> {
        let sql = "INSERT INTO journeys(user_id, lat, lon, created_at) VALUES (?, ?, ?, ?);";
        sqlx::query(sql)
            .bind(user_id)
            .bind(lat)
            .bind(lon)
            .bind(created_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_full_journey_by_user_id(
        &self,
        user_id: i64,
    ) -> Result<Vec<JourneyWaypoint>, sqlx::Error> {
        let sql = "SELECT user_id, lat, lon, created_at FROM journeys WHERE user_id = ? ORDER BY created_at ASC;";
        let journey: Vec<JourneyWaypoint> = sqlx::query_as(sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(journey)
    }
}



#[cfg(test)]
mod tests {
    //use super::*;
    use crate::repository::{journeys_repository::JourneysRepository, users_repository::UsersRepository};
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

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS journeys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            lat REAL NOT NULL,
            lon REAL NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
            );",
        )
        .execute(&pool)
        .await
        .expect("Impossibile creare la tabella journeys");

        pool
    }


    #[tokio::test]
    async fn test_insert_journey_waypoint_success() {
        let pool = setup_db().await;
        let users_repo = UsersRepository::new(pool.clone());
        let journeys_repo = JourneysRepository::new(pool);

        let result: Result<i64, sqlx::Error> = users_repo.insert_user("test", b"pswtest").await;
        if let Ok(user_id) = result {
            let inserted = journeys_repo.insert_journey_waypoint(user_id, 45.4642, 9.1900, "2026-08-05 12:00:00".to_string()).await;
            assert!(inserted.is_ok());
        }
    }

    #[tokio::test] // 1. Aggiunto l'attributo per il test asincrono
    async fn test_get_full_journey_by_user_id_success() {
        let pool = setup_db().await;
        let users_repo = UsersRepository::new(pool.clone());
        let journeys_repo = JourneysRepository::new(pool);

        let result: Result<i64, sqlx::Error> = users_repo.insert_user("test", b"pswtest").await;
        if let Ok(user_id) = result {
            // Usiamo ? anche qui per pulizia, dato che la funzione ora restituisce Result
            journeys_repo.insert_journey_waypoint(user_id, 45.4642, 9.1900, "2026-08-05 12:00:00".to_string()).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1950, "2026-08-05 12:15:00".to_string()).await.unwrap();
            
            let journey = journeys_repo.get_full_journey_by_user_id(user_id).await.unwrap();
            assert_eq!(journey.len(), 2);

            assert_eq!(journey[0].lat, 45.4642);
            // Nota: qui nel tuo codice originale controllavi journey[1].lon con 45.4650 (che era la lat). Corretto in .lat
            assert_eq!(journey[1].lat, 45.4650); 
            assert_eq!(journey[0].created_at, "2026-08-05 12:00:00".to_string());
        }
    }
 
}