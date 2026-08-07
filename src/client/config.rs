use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub coord_file_path: String,
    pub tick_interval_millis: u32,
    pub client_username: String,
    pub client_password: String,
    pub server_url: String,
}

impl Config {
    pub fn load(path: &str) -> Result<Config, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("cannot read config '{}': {}", path, e))?;

        serde_json::from_str(&content)
            .map_err(|e| format!("invalid config '{}': {}", path, e))
    }
}