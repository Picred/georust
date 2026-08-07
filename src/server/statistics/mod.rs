use chrono::{Datelike, Days, Local};
use std::time::Duration;
use crate::repository::journeys_repository::{JourneysRepository};
use super::utils::*;



#[derive(Copy, Clone, Debug)]
pub enum RequiredTimeFrame {
    CurrentDay,
    CurrentWeek,
    CurrentMonth,
}


#[derive(Debug)]
pub struct TimeRange {
    pub start: String,
    pub end: String,
}


impl TimeRange {
    fn new(start: String, end: String) -> Self {
        Self { start, end }
    }
}


pub struct Statistics {
    pub timeframe: RequiredTimeFrame,
    journeys_repository: JourneysRepository
}


impl Statistics {
    pub fn new(timeframe: RequiredTimeFrame, journeys_repository: JourneysRepository) -> Self {
        Self { timeframe, journeys_repository}
    }


    pub fn set_timeframe(&mut self, new_timeframe: RequiredTimeFrame) {
        self.timeframe = new_timeframe;
    }


    pub fn get_timeframe(&self) -> RequiredTimeFrame {
        self.timeframe
    }


    fn convert_timeframe_to_range(&self) -> TimeRange{
        let today_raw = Local::now();

        match self.timeframe {
            RequiredTimeFrame::CurrentDay => {
                let start = format!("{}", today_raw.format("%Y-%m-%d 00:00:00"));
                let end = format!("{}", today_raw.format("%Y-%m-%d 23:59:59"));
                TimeRange::new(start, end)
            }

            RequiredTimeFrame::CurrentWeek => {
                let days_from_monday = today_raw.weekday().number_from_monday() as u64;
                let weekend_days_length = 7;

                let weekend_start = today_raw.checked_sub_days(Days::new(days_from_monday - 1)).unwrap();
                let weekend_end = weekend_start.checked_add_days(Days::new(weekend_days_length - 1)).unwrap();

                let start = format!("{}", weekend_start.format("%Y-%m-%d 00:00:00"));
                let end = format!("{}", weekend_end.format("%Y-%m-%d 23:59:59"));
                TimeRange::new(start, end)
            }

            RequiredTimeFrame::CurrentMonth => {
                let days_in_this_month: u64 = today_raw.num_days_in_month().into();
                let days_from_month_start = today_raw.day() as u64;

                let first_day_of_the_month = today_raw.checked_sub_days(Days::new(days_from_month_start - 1)).unwrap();
                let last_day_of_the_month = first_day_of_the_month.checked_add_days(Days::new(days_in_this_month - 1)).unwrap();

                let start = format!("{}", first_day_of_the_month.format("%Y-%m-%d 00:00:00"));
                let end = format!("{}", last_day_of_the_month.format("%Y-%m-%d 23:59:59"));
                TimeRange::new(start, end)
            }
        }
    }


    pub async fn get_traveled_distance_by_user_id(&mut self, user_id: i64) -> f64 {
        let timerange = self.convert_timeframe_to_range();
        let journeys = self.journeys_repository.get_journey_by_user_id_between_times(user_id, timerange.start, timerange.end).await.unwrap();

        let distance_km = calculate_total_distance_of_journeys(&journeys);

        distance_km
    }


    pub async fn get_average_speed_by_user_id(&self, user_id: i64) -> f64{
        let timerange = self.convert_timeframe_to_range();
        let journeys = self.journeys_repository.get_journey_by_user_id_between_times(user_id, timerange.start, timerange.end).await.unwrap();
        
        let total_distance_km = calculate_total_distance_of_journeys(&journeys);
        let total_hours = calculate_total_hours_of_journeys(&journeys);

        let average_speed = format!("{:.2}", total_distance_km / total_hours).parse::<f64>().unwrap();
        average_speed
    }


    pub async fn get_full_journeys_duration_by_user_id(&self, user_id: i64) -> f64 {
        let timerange = self.convert_timeframe_to_range();
        let journeys = self.journeys_repository.get_journey_by_user_id_between_times(user_id, timerange.start, timerange.end).await.unwrap();

        calculate_total_hours_of_journeys(&journeys)
    }


    pub async fn get_pause_duration_by_user_id(&self, user_id: i64) -> Duration {
        todo!()
    }
}
