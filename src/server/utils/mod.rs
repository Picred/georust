use chrono::{NaiveDateTime, ParseError};
use vincenty_core::distance_from_points;
use crate::models::journey_waypoint::JourneyWaypoint;


pub fn calculate_total_distance_of_journeys(journeys: &Vec<JourneyWaypoint>) -> f64{
    journeys.windows(2).map(|pair| {
        let start_lat = pair[0].lat;
        let start_lon = pair[0].lon;
        let end_lat = pair[1].lat;
        let end_lon = pair[1].lon;

        distance_from_points(start_lat, start_lon, end_lat, end_lon).unwrap()
    }).sum()
}


pub fn convert_sql_to_naive_datetime(sql_datetime: String) -> Result<NaiveDateTime, ParseError>{
    NaiveDateTime::parse_from_str(&sql_datetime, "%Y-%m-%d %H:%M:%S")
}


pub fn calculate_total_hours_of_journeys(journeys: &Vec<JourneyWaypoint>) -> f64{
    let start_created_at = journeys.first().unwrap().created_at.clone();
    let end_created_at = journeys.last().unwrap().created_at.clone();

    let start_time = convert_sql_to_naive_datetime(start_created_at).unwrap();
    let end_time = convert_sql_to_naive_datetime(end_created_at).unwrap();
    
    let total_seconds = (end_time - start_time).num_seconds() as f64;
    let total_hours = total_seconds / 3600.0;

    total_hours
}

