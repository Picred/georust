#[derive(Debug)]
pub struct Coordinates {
    lat: f32,
    lon: f32,
    pos_time: String,
}

impl Coordinates {
    pub fn new(lat: f32, lon: f32, pos_time: String)->Coordinates {
        Coordinates {lat: lat, lon: lon, pos_time: String}
    }
}