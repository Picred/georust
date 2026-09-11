# G19 - Georust

## Overview

The application, written in Rust, consists of a server to which multiple clients (vehicles) can connect. Each client simulates a moving vehicle along geographical coordinates, periodically sending its telemetry data to the server (every 30 seconds by default). The server collects this data and provides tools to calculate detailed statistics on the routes taken.

## Repository Structure

The repository is primarily organized into the following directories:
- `docs/`: contains technical documentation and user manuals for both client and server.
- `src/`: contains all Rust source files for the project.

## Prerequisites

To compile and run the project, you need the Rust toolchain installed, which includes **Cargo**.

## Installation

To install the application, clone the repository and compile in *release* mode:

```bash
git clone <repository_url>
cd G19
cargo build --release
```

## Running the Application

The application consists of two separate binaries: `server` and `client`.

### Starting the Server

Always start the server before any clients:

```bash
cargo run --bin server --release -- [--with-init]
```

### Starting the Client

To start a single client:

```bash
cargo run --bin client --release -- [OPTIONS]
```

By default, the client looks for its settings in the `./config/client_config.json` file. However, any parameter can be overridden via command-line arguments. This is particularly useful, for example, when launching multiple vehicles simultaneously without modifying the configuration file.

**Available Options:**
- `--config <PATH>`: Specifies a custom path for the JSON configuration file.
- `--client-username <NAME>`: Overrides the username for login/registration.
- `--client-password <PASSWORD>`: Overrides the password.
- `--server-url <URL>`: Overrides the server address (e.g., `ws://127.0.0.1:9001`).
- `--tick-interval-millis <MS>`: Changes the interval (in milliseconds) at which each GPS point is sent.

## Quick Start (Usage Example)

1. **Start the server:** Run the server startup command in a terminal.
2. **Start a client:** Run the client startup command in another terminal. The client will connect to the server and start sending its GPS coordinates automatically.
3. **Client Interaction:**
   - Type the `STOP` command in the client terminal to pause sending coordinates.
4. **Server Interaction:** From the server terminal, you can interact by typing:
   - `statistics <vehicle_id> [DAY|WEEK|MONTH]`: calculates distance traveled, average speed for the specified vehicle, total pause duration, and trip duration.
   - `send <user_id> <message>`: sends a text message to a specific vehicle.
   - `broadcast <message>`: sends a message to all currently connected vehicles.

## Running the Demo (Automated Script)

A dedicated bash script is available to quickly launch a complete test environment with the server and multiple clients. 

> [!IMPORTANT]
> Before using the demo script, switch to the specific `demo` branch:
> ```bash
> git checkout demo
> ```

The `./demo.sh` script provides the following commands:

- **Compilation**: Explicitly compiles the binaries in *release* mode.
  ```bash
  ./demo.sh compile
  ```
- **Start**: Launches (compiling if necessary) the server and `N` specified clients. Clients simulate movement by reading predefined routes from the `./data/` folder. The update frequency can be adjusted (default is 30000ms).
  ```bash
  ./demo.sh start <N> [TICK_INTERVAL_MILLIS]
  ```
  
> [!WARNING]
> Currently, a maximum of 10 valid clients can be started, as `./data/` contains coordinates for only 10 routes.

- **Cleanup**: Safely stops and cleans up all client processes started by the demo. This step is also triggered automatically when pressing `CTRL+C` during startup, which also terminates the server.
  ```bash
  ./demo.sh cleanup
  ```

## Documentation

For full manuals with detailed explanations of all configuration parameters, CLI commands, and architectural design choices, please refer to the documents in the [`docs/`](./docs) folder.

## Authors

- [Amedeo Marino](https://github.com/amedeo03)
- [Andrei Stefan](https://github.com/picred)
- [Thimoty Paduraru](https://github.com/timopad)
