use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug)]
pub struct Config {
    pub coord_file_path: String,
    pub tick_interval_millis: u32,
    pub client_username: String,
    pub client_password: String
}

impl Config {
    pub fn load(path: &str) -> Result<Config, String> {

        let content = fs::read_to_string(PathBuf::from(path))
            .map_err(|e| format!("cannot read config '{}': {}", path, e))?;

        let map = parse_kv(&content);

        Ok(Config {
            coord_file_path: get(&map, "coord_file_path")?,

            tick_interval_millis: get(&map, "tick_interval_millis")?
                .parse()
                .map_err(|_| "invalid tick_interval_millis".to_string())?,

            client_username: get(&map, "client_username")?,

            client_password: get(&map, "client_password")?,
        })
    }
}

fn parse_kv(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let mut parts = line.splitn(2, '=');
            let key = parts.next()?.trim().to_string();
            let value = parts.next()?.trim().to_string();
            Some((key, value))
        })
        .collect()
}

fn get(map: &HashMap<String, String>, key: &str) -> Result<String, String> {
    map.get(key)
        .cloned()
        .ok_or_else(|| format!("missing required config key: '{}'", key))
}