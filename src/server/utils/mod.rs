use chrono::{NaiveDateTime, ParseError};
use vincenty_core::{distance_from_coords, distance_from_points};

use crate::models::journey_waypoint::JourneyWaypoint;

pub fn calculate_distance_between_points(start_lat: f64, start_lon: f64, end_lat: f64, end_lon: f64) -> f64{
    distance_from_points(start_lat, start_lon, end_lat, end_lon).unwrap()
}

pub fn calculate_traveled_distance_of_journeys(journeys: &Vec<JourneyWaypoint>) -> f64{
    journeys.windows(2).map(|pair| {
        let start_lat = pair[0].lat;
        let start_lon = pair[0].lon;
        let end_lat = pair[1].lat;
        let end_lon = pair[1].lon;
        calculate_distance_between_points(start_lat, start_lon, end_lat, end_lon)
    }).sum()
}

pub fn convert_sql_to_naive_datetime(sql_datetime: String) -> Result<NaiveDateTime, ParseError>{
    NaiveDateTime::parse_from_str(&sql_datetime, "%Y-%m-%d %H:%M:%S")
}

