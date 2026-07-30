use std::fs::File;
use std::io::{self, BufRead, BufReader, Lines};

use G19::utils::coord::{Coordinates};

pub struct CoordGenerator {
    lines: Lines<BufReader<File>>,
}

impl CoordGenerator {
    
    pub fn init(path: &str) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(CoordGenerator {
            lines: reader.lines(),
        })
    }

    pub fn get_next(&mut self) -> Option<Coordinates> {
        loop {
            let line = self.lines.next()?;

            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut words = line.split_whitespace();
            let lat = words.next().and_then(|s| s.parse::<f32>().ok());
            let lon = words.next().and_then(|s| s.parse::<f32>().ok());

            if let (Some(lat), Some(lon)) = (lat, lon) {
                return Some(Coordinates::new(lat, lon));
            }
        }
    }
}