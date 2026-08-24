use sqlx::{Pool, Sqlite};
use crate::models::journey_waypoint::JourneyWaypoint;

/// Repository responsible for journey waypoints persistence in SQLite.
pub struct JourneysRepository {
    pub pool: Pool<Sqlite>,
}

impl JourneysRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Stores in the db a single journey waipoint of a user-
    /// 
    /// Arguments:
    /// - `user_id`     - of the user who sent the coordinates.
    /// - `lat`         - latitude of coordinates.
    /// - `lon`         - longitude of coordinates.
    /// - `created_at`  - formated timestamp of the user in this specific coordinates.
    /// - `is_stopped`  - state of the user in this journey waypoint.
    pub async fn insert_journey_waypoint(
        &self,
        user_id: i64,
        lat: f64,
        lon: f64,
        created_at: String,
        is_stopped: bool,
    ) -> Result<(), sqlx::Error> {
        let sql = "INSERT INTO journeys(user_id, lat, lon, created_at, is_stopped) VALUES (?, ?, ?, ?, ?);";
        sqlx::query(sql)
            .bind(user_id)
            .bind(lat)
            .bind(lon)
            .bind(created_at)
            .bind(is_stopped)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Returns the Vec of JourneyWaiponts with field `user_id` equal to the parameter inserted in the function
    pub async fn get_full_journey_by_user_id(
        &self,
        user_id: i64,
    ) -> Result<Vec<JourneyWaypoint>, sqlx::Error> {
        let sql = "SELECT user_id, lat, lon, created_at, is_stopped FROM journeys WHERE user_id = ? ORDER BY created_at ASC;";
        let journey: Vec<JourneyWaypoint> = sqlx::query_as(sql)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(journey)
    }


    /// Return the Vec of JourneyWaipoints with field `created_at` between start_time and end_time (limits included)
    pub async fn get_journey_by_user_id_between_times(
        &self,
        user_id: i64,
        start_time: String,
        end_time: String,
    ) -> Result<Vec<JourneyWaypoint>, sqlx::Error> {

        let sql = "SELECT user_id, lat, lon, created_at, is_stopped FROM journeys WHERE user_id = ? AND created_at >= ? AND created_at <= ?;";
        let journey: Vec<JourneyWaypoint> = sqlx::query_as(sql)
            .bind(user_id)
            .bind(start_time)
            .bind(end_time)
            .fetch_all(&self.pool)
            .await?;
        Ok(journey)

    }


    /// returns the duration of pauses (in seconds) of a user in a programmable time interval (defined by start_time and end_time)
    pub async fn get_total_pauses_by_user_id(
        &self,
        user_id: i64,
        start_time: String,
        end_time: String,
    ) -> Result<i64, sqlx::Error> {

        let sql = "
            WITH PreviousData AS (
                SELECT is_stopped, created_at, 
                    LAG(created_at) OVER (ORDER BY created_at) AS previous_time,
                    LAG(is_stopped) OVER (ORDER BY created_at) AS previous_state
                FROM journeys
                WHERE user_id = ? AND created_at >= ? AND created_at <= ?
            )
            SELECT COALESCE(SUM(unixepoch(created_at) - unixepoch(previous_time)), 0) AS total_seconds
            FROM PreviousData
            WHERE is_stopped = 1 AND previous_state = 1;
        ";
        let pauses_duration: i64 = sqlx::query_scalar(sql)
        .bind(user_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await?;

        Ok(pauses_duration)
    }
    
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::users_repository::UsersRepository;
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
            is_stopped INTEGER NOT NULL,
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
            let inserted = journeys_repo.insert_journey_waypoint(user_id, 45.4642, 9.1900, "2026-08-05 12:00:00".to_string(), false).await;
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
            // inserimento di tuple in journeys
            journeys_repo.insert_journey_waypoint(user_id, 45.4642, 9.1900, "2026-08-05 12:00:00".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1950, "2026-08-05 12:15:00".to_string(), false).await.unwrap();
            
            let journey = journeys_repo.get_full_journey_by_user_id(user_id).await.unwrap();
            assert_eq!(journey.len(), 2);
            assert_eq!(journey[0].lat, 45.4642);
            assert_eq!(journey[1].lat, 45.4650); 
            assert_eq!(journey[0].created_at, "2026-08-05 12:00:00".to_string());
        }
    }

    #[tokio::test]
    async fn test_get_journey_by_user_id_between_times_success() {
        let pool = setup_db().await;
        let users_repo = UsersRepository::new(pool.clone());
        let journeys_repo = JourneysRepository::new(pool);

        let result: Result<i64, sqlx::Error> = users_repo.insert_user("test", b"pswtest").await;
        if let Ok(user_id) = result {
            // Usiamo ? anche qui per pulizia, dato che la funzione ora restituisce Result
            journeys_repo.insert_journey_waypoint(user_id, 45.4642, 9.1900, "2026-08-05 12:15:00".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1950, "2026-08-05 12:15:01".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1955, "2026-08-05 12:15:02".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1965, "2026-08-05 12:15:03".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4655, 9.1970, "2026-08-05 12:15:04".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4663, 9.1971, "2026-08-05 12:15:05".to_string(), false).await.unwrap();
            
            let journey = journeys_repo.get_journey_by_user_id_between_times(user_id, "2026-08-05 12:15:01".to_string(), "2026-08-05 12:15:03".to_string()).await.unwrap();
            for journey_waypoint in &journey {
                println!("{:?}", journey_waypoint);
            }

            assert_eq!(journey.len(), 3, "I journey_waypoint non sono 3");
            assert_eq!(journey[0].lon, 9.1950);
            assert_eq!(journey[1].lon, 9.1955); 
            assert_eq!(journey[1].lat, 45.4650); 
            assert_eq!(journey[2].lon, 9.1965);
            assert_eq!(journey[0].created_at, "2026-08-05 12:15:01".to_string());
            assert_eq!(journey[2].created_at, "2026-08-05 12:15:03".to_string());
        }
    }


    #[tokio::test]
    async fn test_get_journey_by_user_id_between_times_wrong_start_time() {
        let pool = setup_db().await;
        let users_repo = UsersRepository::new(pool.clone());
        let journeys_repo = JourneysRepository::new(pool);

        let result: Result<i64, sqlx::Error> = users_repo.insert_user("test", b"pswtest").await;
        if let Ok(user_id) = result {
            // Usiamo ? anche qui per pulizia, dato che la funzione ora restituisce Result
            journeys_repo.insert_journey_waypoint(user_id, 45.4642, 9.1900, "2026-08-05 12:15:00".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1950, "2026-08-05 12:15:01".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1955, "2026-08-05 12:15:02".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1965, "2026-08-05 12:15:03".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4655, 9.1970, "2026-08-05 12:15:04".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4663, 9.1971, "2026-08-05 12:15:05".to_string(), false).await.unwrap();
            
            let journey = journeys_repo.get_journey_by_user_id_between_times(user_id, "stat_time sbagliato".to_string(), "2026-08-05 12:15:03".to_string()).await.unwrap();
            for journey_waypoint in &journey {
                println!("{:?}", journey_waypoint);
            }

            assert_eq!(journey.len(), 0, "Ci sono journey_waypoint quando dovrebbero essere 0");
        }
    }

    #[tokio::test]
    async fn test_get_total_pauses_by_user_id_success() {
        let pool = setup_db().await;
        let users_repo = UsersRepository::new(pool.clone());
        let journeys_repo = JourneysRepository::new(pool);

        let result: Result<i64, sqlx::Error> = users_repo.insert_user("test", b"pswtest").await;
        if let Ok(user_id) = result {
            // Usiamo ? anche qui per pulizia, dato che la funzione ora restituisce Result
            journeys_repo.insert_journey_waypoint(user_id, 45.4642, 9.1900, "2026-08-05 12:15:00".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1950, "2026-08-05 12:15:01".to_string(), true).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1955, "2026-08-05 12:15:02".to_string(), true).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1965, "2026-08-05 12:15:03".to_string(), true).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4655, 9.1970, "2026-08-05 12:15:04".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4663, 9.1971, "2026-08-05 12:15:05".to_string(), false).await.unwrap();
            
            let duration = journeys_repo.get_total_pauses_by_user_id(user_id, "2026-08-05 12:15:00".to_string(), "2026-08-05 12:15:05".to_string()).await.unwrap();
            println!("Pauses duration: {} seconds", duration);

            assert_eq!(duration, 2, "La durata delle pause calcolata è sbagliata");
        }
    }



    #[tokio::test]
    async fn test_get_total_pauses_by_user_id_with_one_stopped() {
        let pool = setup_db().await;
        let users_repo = UsersRepository::new(pool.clone());
        let journeys_repo = JourneysRepository::new(pool);

        let result: Result<i64, sqlx::Error> = users_repo.insert_user("test", b"pswtest").await;
        if let Ok(user_id) = result {
            // Usiamo ? anche qui per pulizia, dato che la funzione ora restituisce Result
            journeys_repo.insert_journey_waypoint(user_id, 45.4642, 9.1900, "2026-08-05 12:15:00".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1950, "2026-08-05 12:15:01".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1955, "2026-08-05 12:15:02".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4650, 9.1965, "2026-08-05 12:15:03".to_string(), true).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4655, 9.1970, "2026-08-05 12:15:04".to_string(), false).await.unwrap();
            journeys_repo.insert_journey_waypoint(user_id, 45.4663, 9.1971, "2026-08-05 12:15:05".to_string(), false).await.unwrap();
            
            let duration = journeys_repo.get_total_pauses_by_user_id(user_id, "2026-08-05 12:15:00".to_string(), "2026-08-05 12:15:05".to_string()).await.unwrap();
            println!("Pauses duration: {} seconds", duration);

            assert_eq!(duration, 0, "La durata delle pause calcolata è sbagliata");
        }
    }

}