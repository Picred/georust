use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct JourneyWaypoint {
    pub user_id: i64,
    pub lat: f64,
    pub lon: f64,
    pub created_at: String
}

impl JourneyWaypoint {
    pub fn new(user_id: i64, lat: f64, lon: f64, created_at: String) -> Self{
        Self{user_id, lat, lon, created_at}
    }
}