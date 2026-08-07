use serde::Deserialize;
use std::fs;
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
    #[arg(short, long, default_value = "./config/client_config.json")]
    pub config: String,

    /// Override server URL, e.g. ws://127.0.0.1:9001
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
        let content = fs::read_to_string(&cli.config)
            .map_err(|e| format!("cannot read config '{}': {}", cli.config, e))?;

        let mut config: Config = serde_json::from_str(&content)
            .map_err(|e| format!("invalid config '{}': {}", cli.config, e))?;

        config.apply_overrides(cli);

        Ok(config)
    }

    /// Manually apply config overrides from passed cli args
    fn apply_overrides(&mut self, cli: &Cli) {
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
