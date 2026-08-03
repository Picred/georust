#[derive(Debug)]
pub struct Coordinates {
    lat: f32,
    lon: f32,
    created_at: String,
}

impl Coordinates {
    pub fn new(lat: f32, lon: f32, pos_time: String)->Coordinates {
        Coordinates {lat: lat, lon: lon, created_at: pos_time,}
    }
}