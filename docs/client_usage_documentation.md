# Client Usage Documentation

This guide explains how to start and use the Georust client: a program that periodically sends the device location to the Georust server.

## What the Client Does

Once started, the client:

- Connects to the Georust server.
- Logs in with your username and password (or automatically registers if an account does not exist yet).
- Once transmission is started, sends the device location to the server at regular intervals until paused or terminated by the user.
- Allows sending and receiving text messages from the server.

## Configuration File

The JSON configuration file specifies the server address, how frequently to send coordinates, and access credentials. By default, the client looks for this file at `./config/client_config.json`:

```json
{
  "server_url": "ws://127.0.0.1:9001",
  "coord_file_path": "./data/positions.txt",
  "tick_interval_millis": 5000,
  "client_username": "veicolo_test1",
  "client_password": "password_test"
}
```

| Field                       | Meaning                                                                     |
|-----------------------------|-----------------------------------------------------------------------------|
| `server_url`                | Address of the Georust server to connect to.                                |
| `coord_file_path`           | Path to the file containing positions to send.                              |
| `tick_interval_millis`      | Frequency (in milliseconds) at which a new position is sent once started. Example: `5000` = every 5 seconds. |
| `client_username`           | Username used for login (or registration if account doesn't exist yet).      |
| `client_password`           | Password used for login (or registration).                                  |

## Command-Line Options

The client accepts optional command-line parameters, useful for temporarily overriding values without modifying the configuration file. None of them are mandatory; if not passed, the client uses values from the configuration file.

| Parameter                      | Description                                                              |
|--------------------------------|--------------------------------------------------------------------------|
| `--config <path>`              | Path to configuration file to use. Default value is `./config/client_config.json`. |
| `coord_file_path`              | Overrides path to file containing positions to send.                     |
| `--server-url <url>`           | Overrides server address specified in config file for this execution.     |
| `--client-username <name>`     | Overrides username specified in config file for this execution.          |
| `--client-password <password>` | Overrides password specified in config file for this execution.          |
| `--tick-interval-millis <ms>`  | Overrides position sending frequency (in ms) for this execution.         |

## Interactive Console Usage

Once the client is running and connected, the user can type commands directly in the terminal window.

| Command            | Description                                                                    |
|--------------------|--------------------------------------------------------------------------------|
| `stop`             | Pauses position transmission. You can type `start` later to resume.             |
| `start`            | Starts sending your device position to the server.                             |
| `send <message>`   | Sends a custom text message directly to the server. Replace `<message>` with your text, e.g. `send hello`. |
| `exit`             | Closes the client and terminates the connection.                               |
