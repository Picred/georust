use G19::utils::coordinates::Coordinates;
use super::utils::convert_sql_to_naive_datetime;

/// Handler is used to manage the calculation of the user's state during the user's session (lifetime of this handler).
/// 
/// Fields:
/// - `previous_coordinates`     -> useful to know if the new coordinates are changed from the last stored in db for that user
/// - `sum_continuous_stop_time` -> counts the number of seconds that have passed since the first waipont journey in which the user is in the STOP state (it is reset when the user enters the MOVE state)
/// - `first_eq_coordinates`     -> flag witch is used to understand if the coordinates sent have changed since the beginning of the session (true = not changed, false = changed)
pub struct UserStateHandler{
    previous_coordinates: Option<Coordinates>,
    sum_continuous_stop_time: i64,
    first_eq_coordinates: bool,
}


impl UserStateHandler {
    // By default, a user starts with the "STATIONARY" state (which corresponds to true in the is_stopped field in journeys)
    const DEFAULT_USER_STATE: bool = true; 
    // 3 minutes during which coordinates remain the same to transition from "IN_MOTION" to "STATIONARY" state
    const MOTIONLESS_THRESHOLD_SECS: i64 = 180; 

    pub fn new() -> Self {
        Self{
            previous_coordinates: None,
            sum_continuous_stop_time: 0,
            first_eq_coordinates: true,
        }
    }

    /// Calculate the state of the user, based on the new coordinates that have been received (argument to insert) and what happened in the past.
    pub fn calculate_user_state(&mut self, new_coordinates: Coordinates) -> bool {
        let user_state = if let Some(old_coordinates) = &self.previous_coordinates {
            if self.first_eq_coordinates && Self::DEFAULT_USER_STATE && old_coordinates.lat == new_coordinates.lat && old_coordinates.lon == new_coordinates.lon {
                true
            } else if old_coordinates.lat == new_coordinates.lat && old_coordinates.lon == new_coordinates.lon {
                self.add_interval_in_stop_time( 
                    old_coordinates.created_at.clone(),
                    new_coordinates.created_at.clone());
                if self.sum_continuous_stop_time >= Self::MOTIONLESS_THRESHOLD_SECS { true } else { false }
            } else {
                self.first_eq_coordinates = false;
                self.sum_continuous_stop_time = 0;
                false
            }
        } else {
            Self::DEFAULT_USER_STATE
        };
        self.previous_coordinates = Some(new_coordinates);
        user_state
    }

    /// Add to `sum_continuous_stop_time` the interval (in secs) between the new_datetime and the previous_datetime 
    fn add_interval_in_stop_time(&mut self, previous_datetime: String, new_datetime: String) {

        let pre_t = convert_sql_to_naive_datetime(previous_datetime).unwrap();
        let new_t = convert_sql_to_naive_datetime(new_datetime).unwrap();

        let interval = (new_t - pre_t).num_seconds() as i64;
        self.sum_continuous_stop_time += interval;
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create mock coordinates with a specific timestamp
    fn create_mock_coords(lat: f64, lon: f64, time_str: &str) -> Coordinates {
        Coordinates {
            lat,
            lon,
            created_at: time_str.to_string(),
        }
    }

    #[test]
    fn test_initial_state_returns_default() {
        let mut handler = UserStateHandler::new();
        let coords = create_mock_coords(45.0, 7.0, "2026-01-01 10:00:00");

        // Upon the first insertion, it must return the default value (true = STATIONARY)
        let state = handler.calculate_user_state(coords);
        assert_eq!(state, UserStateHandler::DEFAULT_USER_STATE);
        assert_eq!(handler.sum_continuous_stop_time, 0);
    }

    #[test]
    fn test_from_stop_to_moving() {
        let mut handler = UserStateHandler::new();
        
        // First point (Initial state: STATIONARY by default)
        let coords1 = create_mock_coords(45.0, 7.0, "2026-01-01 10:00:00");
        handler.calculate_user_state(coords1);

        // Second point with different coordinates (It is moving -> false)
        let coords2 = create_mock_coords(45.1, 7.1, "2026-01-01 10:01:00");
        let state = handler.calculate_user_state(coords2);

        assert!(!state); // Must be false (IN_MOTION)
        assert_eq!(handler.sum_continuous_stop_time, 0);
    }

    #[test]
    fn test_accumulation_of_stop_time_and_threshold() {
        let mut handler = UserStateHandler::new();

        // 1. Initialization (Returns true by default, time = 0)
        let c1 = create_mock_coords(45.0, 7.0, "2026-01-01 10:00:00");
        handler.calculate_user_state(c1);

        // 2. Moves to change the state to false (IN_MOTION)
        let c2 = create_mock_coords(45.1, 7.1, "2026-01-01 10:01:00");
        handler.calculate_user_state(c2);

        // 3. Remains stationary at the new location for 60 seconds (Below the 180s threshold)
        // New coordinates identical to c2, time +60 seconds
        let c3 = create_mock_coords(45.1, 7.1, "2026-01-01 10:02:00");
        let state_after_c3 = handler.calculate_user_state(c3);
        
        assert_eq!(state_after_c3, false); // Still moving because it has not exceeded the threshold
        assert_eq!(handler.sum_continuous_stop_time, 60);

        println!("sum_continuous_stop_time: {}", handler.sum_continuous_stop_time);

        // 4. Remains stationary for another 130 seconds at the same position (Total stop time: 190s > 180s)
        // New coordinates identical to c3, time +130 seconds
        let c4 = create_mock_coords(45.1, 7.1, "2026-01-01 10:04:10");
        let state_after_c4 = handler.calculate_user_state(c4);
        println!("sum_continuous_stop_time: {}", handler.sum_continuous_stop_time);
        assert_eq!(state_after_c4, true); // Now it must be true (STATIONARY) because it exceeded 180s
        assert_eq!(handler.sum_continuous_stop_time, 190);
    }

    #[test]
    fn test_reset_after_moving_again() {
        let mut handler = UserStateHandler::new();

        // Send a sequence that exceeds the stop time threshold
        handler.calculate_user_state(create_mock_coords(45.0, 7.0, "2026-01-01 10:00:00"));
        handler.calculate_user_state(create_mock_coords(45.0, 7.0, "2026-01-01 10:05:00")); // +300 seconds -> State: STATIONARY

        // Now the user moves off and changes coordinates
        let moving_coords = create_mock_coords(45.5, 7.5, "2026-01-01 10:06:00");
        let state = handler.calculate_user_state(moving_coords);

        // The state must reset immediately to false and the counter to 0
        assert_eq!(state, false);
        assert_eq!(handler.sum_continuous_stop_time, 0);
    }

    #[test]
    fn test_first_eq_coordinates_stop() {
        let mut handler = UserStateHandler::new();

        handler.calculate_user_state(create_mock_coords(45.0, 7.0, "2026-01-01 10:00:00"));
        
        let state2 = handler.calculate_user_state(create_mock_coords(45.0, 7.0, "2026-01-01 10:01:00"));
        assert_eq!(state2, true);

        let state3 = handler.calculate_user_state(create_mock_coords(45.0, 7.0, "2026-01-01 10:02:00"));
        assert_eq!(state3, true);

        let state3 = handler.calculate_user_state(create_mock_coords(45.1, 7.0, "2026-01-01 10:02:00"));
        assert_eq!(state3, false);
        assert_eq!(handler.sum_continuous_stop_time, 0);
    }
}
