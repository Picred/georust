use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct JourneyWaypoint {
    pub id: i32,
    pub user_id: i32,
    pub lat: f64,
    pub lon: f64,
    pub pos_time: String,
}

impl JourneyWaypoint {
    pub fn new(id: i32, user_id: i32, lat: f64, lon: f64, pos_time: String) -> Self{
        Self{id, user_id, lat, lon, pos_time}
    }
}