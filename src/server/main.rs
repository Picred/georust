// DEVELOPMENT
// -----------------------
// Non cancellabile
pub mod authenticator;
pub mod database;
pub mod models;
pub mod repository;
pub mod statistics;
// -----------------------
pub mod connection_manager;
pub mod journey_tracking;

use std::env::args;

use crate::statistics::{Statistics, RequiredTimeFrame};

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let _args: Vec<String> = args().collect();

    let stats = Statistics::new(RequiredTimeFrame::CurrentDay);
    // let mut stats = Statistics::new(RequiredTimeFrame::CurrentWeek);
    // let mut stats = Statistics::new(RequiredTimeFrame::CurrentMonth);

    // println!("Initial timeframe {:?}", stats.get_timeframe());
    // stats.set_timeframe(RequiredTimeFrame::CurrentWeek);
    // println!("Updated timeframe {:?}", stats.get_timeframe());


    let range = stats.convert_timeframe_to_range();

    println!("{:?}", range);
    Ok(())
}

