#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Coordinates {
    pub lat: f64,
    pub lon: f64,
    pub created_at: String,
}

impl Coordinates {
    pub fn new(lat: f64, lon: f64, pos_time: String)->Coordinates {
        Coordinates {lat: lat, lon: lon, created_at: pos_time,}
    }
}