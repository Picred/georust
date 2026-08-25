use std::fs;

use serde::Deserialize;
use clap::Parser;

/// In-memory version of the config file used during program execution
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Path to the JSON config file
    pub coord_file_path: String,

    /// server websocket address
    pub server_url: String,

    /// interval between coordinates messages
    pub tick_interval_millis: u32,

    pub client_username: String,
    pub client_password: String
}

/// Command-line overrides for client config.
/// All fields are optional - anything not passed falls back to the config file.
#[derive(Parser, Debug)]
pub struct Cli {
    /// Path to the JSON config file
    #[arg(long, default_value = "./config/client_config.json")]
    pub config_path: String,

    /// Override coordinates file path
    #[arg(long)]
    pub coord_file_path: Option<String>,

    /// Override server URL
    #[arg(long)]
    pub server_url: Option<String>,

    /// Override client username
    #[arg(long)]
    pub client_username: Option<String>,

    /// Override client password
    #[arg(long)]
    pub client_password: Option<String>,

    /// Override tick interval in milliseconds
    #[arg(long)]
    pub tick_interval_millis: Option<u32>,
}

impl Config {
    /// Loads config values from cli params.
    /// Cli params will always have priority over the config file.
    pub fn load(cli: &Cli) -> Result<Config, String> {
        let content = fs::read_to_string(&cli.config_path)
            .map_err(|e| format!("cannot read config '{}': {}", cli.config_path, e))?;

        let mut config: Config = serde_json::from_str(&content)
            .map_err(|e| format!("invalid config '{}': {}", cli.config_path, e))?;

        config.apply_overrides(cli);

        Ok(config)
    }

    /// Apply config overrides from passed cli args
    fn apply_overrides(&mut self, cli: &Cli) {
        if let Some(ref coord_file_path) = cli.coord_file_path {
            self.coord_file_path = coord_file_path.clone();
        }
        if let Some(ref server_url) = cli.server_url {
            self.server_url = server_url.clone();
        }
        if let Some(ref username) = cli.client_username {
            self.client_username = username.clone();
        }
        if let Some(ref password) = cli.client_password {
            self.client_password = password.clone();
        }
        if let Some(tick) = cli.tick_interval_millis {
            self.tick_interval_millis = tick;
        }
    }
}
