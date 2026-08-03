#[derive(Debug)]
pub struct Coordinates {
    lat: f32,
    lon: f32,
}

impl Coordinates {
    pub fn new(lat: f32, lon: f32)->Coordinates {
        Coordinates {lat: lat, lon: lon}
    }
}