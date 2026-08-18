use G19::utils::coordinates::Coordinates;
use super::utils::convert_sql_to_naive_datetime;

pub struct UserStateHandler{
    previous_coordinates: Option<Coordinates>,
    sum_continuous_stop_time: i64,
    first_eq_coordinates: bool,
}


impl UserStateHandler {
    const DEFAULT_USER_STATE: bool = true; // di default un user inizia con lo stato "FERMO" (che corrisponde a true del del campo is_stopped in journeys)
    const MOTIONLESS_THRESHOLD_SECS: i64 = 180; // 3 minuti in cui le coordinate rimangono uguali per passare da stato "IN_MOVIMENTO" a "fermo"

    pub fn new() -> Self {
        Self{
            previous_coordinates: None,
            sum_continuous_stop_time: 0,
            first_eq_coordinates: true,
        }
    }


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

    // Funzione helper per creare coordinate fittizie con un timestamp specifico
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

        // Al primo inserimento deve restituire il valore di default (true = FERMO)
        let state = handler.calculate_user_state(coords);
        assert_eq!(state, UserStateHandler::DEFAULT_USER_STATE);
        assert_eq!(handler.sum_continuous_stop_time, 0);
    }

    #[test]
    fn test_from_stop_to_moving() {
        let mut handler = UserStateHandler::new();
        
        // Primo punto (Stato iniziale: FERMO di default)
        let coords1 = create_mock_coords(45.0, 7.0, "2026-01-01 10:00:00");
        handler.calculate_user_state(coords1);

        // Secondo punto con coordinate diverse (Si sta muovendo -> false)
        let coords2 = create_mock_coords(45.1, 7.1, "2026-01-01 10:01:00");
        let state = handler.calculate_user_state(coords2);

        assert!(!state); // Deve essere false (IN_MOVIMENTO)
        assert_eq!(handler.sum_continuous_stop_time, 0);
    }

    #[test]
    fn test_accumulation_of_stop_time_and_threshold() {
        let mut handler = UserStateHandler::new();

        // 1. Inizializzazione (Ritorna true di default, tempo = 0)
        let c1 = create_mock_coords(45.0, 7.0, "2026-01-01 10:00:00");
        handler.calculate_user_state(c1);

        // 2. Si muove per cambiare lo stato in false (IN_MOVIMENTO)
        let c2 = create_mock_coords(45.1, 7.1, "2026-01-01 10:01:00");
        handler.calculate_user_state(c2);

        // 3. Rimane fermo nella nuova posizione per 60 secondi (Sotto la soglia di 180s)
        // Nuove coordinate uguali a c2, tempo +60 secondi
        let c3 = create_mock_coords(45.1, 7.1, "2026-01-01 10:02:00");
        let state_after_c3 = handler.calculate_user_state(c3);
        
        assert_eq!(state_after_c3, false); // Ancora in movimento perché non ha superato la soglia
        assert_eq!(handler.sum_continuous_stop_time, 60);

        println!("sum_continuous_stop_time: {}", handler.sum_continuous_stop_time);

        // 4. Rimane fermo per altri 130 secondi nella stessa posizione (Totale sosta: 190s > 180s)
        // Nuove coordinate uguali a c3, tempo +130 secondi
        let c4 = create_mock_coords(45.1, 7.1, "2026-01-01 10:04:10");
        let state_after_c4 = handler.calculate_user_state(c4);
        println!("sum_continuous_stop_time: {}", handler.sum_continuous_stop_time);
        assert_eq!(state_after_c4, true); // Ora deve essere true (FERMO) perché ha superato i 180s
        assert_eq!(handler.sum_continuous_stop_time, 190);
    }

    #[test]
    fn test_reset_after_moving_again() {
        let mut handler = UserStateHandler::new();

        // Mandiamo una sequenza che supera la soglia di sosta
        handler.calculate_user_state(create_mock_coords(45.0, 7.0, "2026-01-01 10:00:00"));
        handler.calculate_user_state(create_mock_coords(45.0, 7.0, "2026-01-01 10:05:00")); // +300 secondi -> Stato: FERMO

        // Adesso l'utente riparte e cambia coordinate
        let moving_coords = create_mock_coords(45.5, 7.5, "2026-01-01 10:06:00");
        let state = handler.calculate_user_state(moving_coords);

        // Lo stato deve resettarsi immediatamente a false e il contatore a 0
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
