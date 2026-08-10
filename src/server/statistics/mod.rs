use chrono::{Datelike, Days, Local};
use crate::repository::journeys_repository::{JourneysRepository};
use super::utils::*;



#[derive(Copy, Clone, Debug)]
pub enum RequiredTimeFrame {
    CurrentDay,
    CurrentWeek,
    CurrentMonth,
}


#[derive(Debug)]
pub struct TimeRange {
    pub start: String,
    pub end: String,
}


impl TimeRange {
    fn new(start: String, end: String) -> Self {
        Self { start, end }
    }
}


pub struct Statistics {
    pub timeframe: RequiredTimeFrame,
    journeys_repository: JourneysRepository
}


impl Statistics {
    pub fn new(timeframe: RequiredTimeFrame, journeys_repository: JourneysRepository) -> Self {
        Self { timeframe, journeys_repository}
    }


    pub fn set_timeframe(&mut self, new_timeframe: RequiredTimeFrame) {
        self.timeframe = new_timeframe;
    }


    pub fn get_timeframe(&self) -> RequiredTimeFrame {
        self.timeframe
    }


    fn convert_timeframe_to_range(&self) -> TimeRange{
        let today_raw = Local::now();

        match self.timeframe {
            RequiredTimeFrame::CurrentDay => {
                let start = format!("{}", today_raw.format("%Y-%m-%d 00:00:00"));
                let end = format!("{}", today_raw.format("%Y-%m-%d 23:59:59"));
                TimeRange::new(start, end)
            }

            RequiredTimeFrame::CurrentWeek => {
                let days_from_monday = today_raw.weekday().number_from_monday() as u64;
                let weekend_days_length = 7;

                let weekend_start = today_raw.checked_sub_days(Days::new(days_from_monday - 1)).unwrap();
                let weekend_end = weekend_start.checked_add_days(Days::new(weekend_days_length - 1)).unwrap();

                let start = format!("{}", weekend_start.format("%Y-%m-%d 00:00:00"));
                let end = format!("{}", weekend_end.format("%Y-%m-%d 23:59:59"));
                TimeRange::new(start, end)
            }

            RequiredTimeFrame::CurrentMonth => {
                let days_in_this_month: u64 = today_raw.num_days_in_month().into();
                let days_from_month_start = today_raw.day() as u64;

                let first_day_of_the_month = today_raw.checked_sub_days(Days::new(days_from_month_start - 1)).unwrap();
                let last_day_of_the_month = first_day_of_the_month.checked_add_days(Days::new(days_in_this_month - 1)).unwrap();

                let start = format!("{}", first_day_of_the_month.format("%Y-%m-%d 00:00:00"));
                let end = format!("{}", last_day_of_the_month.format("%Y-%m-%d 23:59:59"));
                TimeRange::new(start, end)
            }
        }
    }


    pub async fn get_traveled_distance_by_user_id(&mut self, user_id: i64) -> Result<f64, sqlx::Error> {
        let timerange = self.convert_timeframe_to_range();
        let journeys = self.journeys_repository.get_journey_by_user_id_between_times(user_id, timerange.start, timerange.end).await?;

        if journeys.len() < 2 {
            return Ok(0.0);
        }

        let distance_km = calculate_total_distance_of_journeys(&journeys);

        Ok(distance_km)
    }


    pub async fn get_average_speed_by_user_id(&self, user_id: i64) -> Result<f64, sqlx::Error>{
        let timerange = self.convert_timeframe_to_range();
        let journeys = self.journeys_repository.get_journey_by_user_id_between_times(user_id, timerange.start, timerange.end).await?;

        if journeys.len() < 2 {
            return Ok(0.0);
        }
    
        let total_distance_km = calculate_total_distance_of_journeys(&journeys);
        let total_hours = calculate_total_hours_of_journeys(&journeys);

        let average_speed = format!("{:.2}", total_distance_km / total_hours).parse::<f64>().unwrap();
        
        Ok(average_speed)
    }


    pub async fn get_full_journeys_duration_by_user_id(&self, user_id: i64) -> Result<f64, sqlx::Error> {
        let timerange = self.convert_timeframe_to_range();
        let journeys = self.journeys_repository.get_journey_by_user_id_between_times(user_id, timerange.start, timerange.end).await?;

        if journeys.len() < 2 {
            return Ok(0.0);
        }

        Ok(calculate_total_hours_of_journeys(&journeys))
    }


    pub async fn get_pauses_hours_by_user_id(&self, user_id: i64) -> Result<f64, sqlx::Error> {
        let timerange = self.convert_timeframe_to_range();
        let total_pauses_seconds = self.journeys_repository.get_total_pauses_by_user_id(user_id, timerange.start, timerange.end).await? as f64;

        let seconds_to_hours_divider = 3600.0;
        let total_pauses_hours = format!("{:.2}", total_pauses_seconds / seconds_to_hours_divider).parse::<f64>().unwrap();

        Ok(total_pauses_hours)
    }
}







#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

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
        .expect("impossibile creare la tabella users");

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
        .expect("impossibile creare la tabella journeys");

        // Inserimento utente di test per rispettare la Foreign Key
        sqlx::query("INSERT INTO users (username, password) VALUES ('test_user', 'psw');")
            .execute(&pool)
            .await
            .expect("impossibile creare utente di test");

        pool
    }



    #[tokio::test]
    async fn test_get_traveled_distance_insufficient_waypoints() {
        let pool = setup_db().await;
        let repo = JourneysRepository::new(pool);
        let mut stats = Statistics::new(RequiredTimeFrame::CurrentDay, repo);

        let distance = stats.get_traveled_distance_by_user_id(1).await.unwrap();
        assert_eq!(distance, 0.0);
    }

    #[tokio::test]
    async fn test_get_traveled_distance_success() {
        let pool = setup_db().await;
        let repo = JourneysRepository::new(pool);
        let now = Local::now();
        let time_1 = now.format("%Y-%m-%d 10:00:00").to_string();
        let time_2 = now.format("%Y-%m-%d 10:30:00").to_string();
        repo.insert_journey_waypoint(1, 45.4642, 9.1900, time_1, false).await.unwrap();
        repo.insert_journey_waypoint(1, 45.4650, 9.1950, time_2, false).await.unwrap();
        let mut stats = Statistics::new(RequiredTimeFrame::CurrentDay, repo);

        let distance = stats.get_traveled_distance_by_user_id(1).await;

        assert!(distance.is_ok());
    }


    #[tokio::test]
    async fn test_get_average_speed_insufficient_waypoints() {
        let pool = setup_db().await;
        let repo = JourneysRepository::new(pool);
        let stats = Statistics::new(RequiredTimeFrame::CurrentDay, repo);

        let speed = stats.get_average_speed_by_user_id(1).await.unwrap();

        assert_eq!(speed, 0.0);
    }

    #[tokio::test]
    async fn test_get_average_speed_success() {
        let pool = setup_db().await;
        let repo = JourneysRepository::new(pool);
        let now = Local::now();
        let time_1 = now.format("%Y-%m-%d 10:00:00").to_string();
        let time_2 = now.format("%Y-%m-%d 11:00:00").to_string();
        repo.insert_journey_waypoint(1, 45.4642, 9.1900, time_1, false).await.unwrap();
        repo.insert_journey_waypoint(1, 45.4742, 9.2000, time_2, false).await.unwrap();
        let stats = Statistics::new(RequiredTimeFrame::CurrentDay, repo);

        let speed = stats.get_average_speed_by_user_id(1).await;

        assert!(speed.is_ok());
    }


    #[tokio::test]
    async fn test_get_full_journeys_duration_insufficient_waypoints() {
        let pool = setup_db().await;
        let repo = JourneysRepository::new(pool);
        let stats = Statistics::new(RequiredTimeFrame::CurrentDay, repo);

        let duration = stats.get_full_journeys_duration_by_user_id(1).await.unwrap();

        assert_eq!(duration, 0.0);
    }

    #[tokio::test]
    async fn test_get_full_journeys_duration_success() {
        let pool = setup_db().await;
        let repo = JourneysRepository::new(pool);
        let now = Local::now();
        let time_1 = now.format("%Y-%m-%d 10:00:00").to_string();
        let time_2 = now.format("%Y-%m-%d 12:00:00").to_string(); // Differenza esatta di 2 ore
        repo.insert_journey_waypoint(1, 45.4642, 9.1900, time_1, false).await.unwrap();
        repo.insert_journey_waypoint(1, 45.4650, 9.1950, time_2, false).await.unwrap();
        let stats = Statistics::new(RequiredTimeFrame::CurrentDay, repo);

        let duration = stats.get_full_journeys_duration_by_user_id(1).await.unwrap();

        assert_eq!(duration, 2.0);
    }


    #[tokio::test]
    async fn test_get_pauses_hours_zero_pauses() {
        let pool = setup_db().await;
        let repo = JourneysRepository::new(pool);
        let stats = Statistics::new(RequiredTimeFrame::CurrentDay, repo);

        let pauses = stats.get_pauses_hours_by_user_id(1).await.unwrap();

        assert_eq!(pauses, 0.0);
    }

    #[tokio::test]
    async fn test_get_pauses_hours_success() {
        let pool = setup_db().await;
        let repo = JourneysRepository::new(pool);
        let now = Local::now();
        let time_1 = now.format("%Y-%m-%d 10:00:00").to_string();
        let time_2 = now.format("%Y-%m-%d 12:00:00").to_string(); // 7200 secondi in pausa = 2.00 ore
        repo.insert_journey_waypoint(1, 45.4642, 9.1900, time_1, true).await.unwrap();
        repo.insert_journey_waypoint(1, 45.4642, 9.1900, time_2, true).await.unwrap();
        let stats = Statistics::new(RequiredTimeFrame::CurrentDay, repo);

        let pauses = stats.get_pauses_hours_by_user_id(1).await.unwrap();

        assert_eq!(pauses, 2.00);
    }
}