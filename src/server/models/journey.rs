use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct Journey {
    pub id: i32,
    pub user_id: i32,
    pub lat: f64,
    pub lon: f64,
    pub created_at: String
}

impl Journey{
    pub fn new(id: i32, user_id: i32, lat: f64, lon: f64, created_at: String) -> Self{
        Self{id, user_id, lat, lon, created_at}
    }
}