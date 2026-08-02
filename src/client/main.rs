use std::{thread, time};

mod coord_gen;
use crate::coord_gen::CoordGenerator;

mod config;
use config::Config;


fn main() -> Result<(), Box<dyn std::error::Error>> {

    let cfg = Config::load("./config/client_config.txt")
        .map_err(|e| format!("Error while parsing client config file: {} ", e))?;

    println!("{:?}", cfg);

    let mut generator = CoordGenerator::init(&cfg.coord_file_path)
        .map_err(|e| format!("Error while creating CoordGenerator: {} ", e))?;
    
    while let Some(coord) = generator.get_next() {
        println!("{coord:?}");
        thread::sleep(time::Duration::from_millis(cfg.tick_interval_millis.into()));
    }

    Ok(())
}