mod coord_gen;
use std::{thread, time};

use crate::coord_gen::CoordGenerator;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut generator = CoordGenerator::init("./data/coordinates.txt")?;
    
    while let Some(coord) = generator.get_next() {
        println!("{coord:?}");
        thread::sleep(time::Duration::from_millis(30_000));
    }

    Ok(())
}