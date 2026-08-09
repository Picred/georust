use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct JourneyWaypoint {
    pub user_id: i64,
    pub lat: f64,
    pub lon: f64,
    pub created_at: String,
    pub is_stopped: bool,
}

impl JourneyWaypoint {
    pub fn new(user_id: i64, lat: f64, lon: f64, created_at: String, is_stopped: bool) -> Self{
        Self{user_id, lat, lon, created_at, is_stopped,}
    }
}