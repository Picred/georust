#[derive(Debug)]
pub struct Coordinates {
    lat: f64,
    lon: f64,
    created_at: String,
}

impl Coordinates {
    pub fn new(lat: f64, lon: f64, pos_time: String)->Coordinates {
        Coordinates {lat: lat, lon: lon, created_at: pos_time,}
    }
}