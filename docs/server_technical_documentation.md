# Server Technical Documentation

The server, through Tokio tasks, handles incoming client connections and offers a usable CLI to send messages to clients (1...N) and request specific statistics for a given client.

> [!NOTE]
> This document explores the internal architecture, modules, and implementation choices of the server. For practical instructions on installation, execution, and CLI command usage, consult the **[Server Usage Documentation](./server_usage_documentation.md)**.

## Main Crates (Dependencies)

The project leverages Rust's asynchronous ecosystem relying on the following primary crates:
- **`tokio`**: asynchronous runtime used for multitasking and concurrent connection management.
- **`tokio-tungstenite`**: library for asynchronous implementation of the WebSocket protocol.
- **`sqlx`**: for asynchronous and safe interaction (via parameterized queries and connection pools) with the SQLite database.
- **`argon2`**: cryptographic standard for secure password hashing.
- **`serde_json`**: for fast serialization and deserialization of exchanged packets.

## Structure and Functionality of the Server Module

```bash
src/server
├── main.rs
├── authenticator
├── connection_manager
├── database
│   └── database.sqlite
├── models
│   └── journey_waypoint.rs
├── repository
│   ├── journeys_repository.rs
│   ├── server_state.rs
│   └── users_repository.rs
├── server_messaging
├── statistics
├── user_session_handler
├── user_state_handler
└── utils
```

### Connection Manager

The connection manager module coordinates incoming network connections (via WebSocket) and the server's administrative console (CLI). To do this, it uses the `ConnectionManager` struct, which maintains an in-memory global map (`sockets`) protected by a `RwLock`, mapping each client to a unique identifier (`Uuid`).

The primary data structures used in this module are:

- **ActiveSocket**: represents a single active socket in memory. It contains the `socket_id` (the unique ID generated upon connection acceptance), the optional `user_id` field (which remains `None` until the user logs in), and a channel `tx` of type `mpsc::Sender` used to send messages to the client.
  
- **ServerState**: represents the global state of the server repositories. Inside, it unifies instances of `JourneysRepository` (`journeys_repo`) and `UsersRepository` (`users_repo`) starting from a single SQLite connection pool (`Pool<Sqlite>`). It is wrapped in a smart pointer `Arc` inside the Connection Manager to allow safe and concurrent database access from all asynchronous tasks.

- **AuthRequest / AuthResponse**: these are JSON serialized and deserialized data structures used to exchange credentials and outcomes during the initial login or registration phase.

#### Execution Flow (run Method)

Upon server startup, the `run` method acts as a task dispatcher launching two concurrent asynchronous flows simultaneously:

1. **Administrative CLI**: spawns a background task that continuously reads commands typed by the administrator on the server console. It allows using the commands `statistics <user_id> [DAY|WEEK|MONTH]` (to print the vehicle report), `send <user_id> <message>` (for a private message), `broadcast <message>` (for a broadcast message to all clients), and `help`.

2. **Accept Loop**: enters an infinite loop on the `TcpListener` to accept incoming connections. For each accepted client, it generates a new `Uuid` (the `socket_id`), logs the event, and spawns an independent task via `tokio::spawn` calling the `handle_connection` function.

#### Connection Lifecycle (handle_connection Method)

The `handle_connection` function isolates and executes the initial logic for each individual connection following these steps:

- **Handshake and Split**: performs the asynchronous WebSocket handshake via `tokio-tungstenite` and splits the stream into two separate channels for reading (`ws_receiver`) and writing (`ws_sender`).

- **Initialization and Writing**: creates an internal `mpsc` channel with a capacity of 100 messages and inserts the `ActiveSocket` struct into the global map of active sockets (with `user_id = None`). Immediately after, it spawns a dedicated micro-task listening on the internal channel to pull messages and physically write them to the network via `ws_sender`.

- **Authentication Phase**: enters a message receiving loop for messages sent by the client. Handles premature disconnections if a `Close` message is received. If valid JSON text (`AuthRequest`) is received, it checks the `action` field:
  - **login**: validates credentials via `users_repo`. If successful, it updates the map inserting the correct `user_id` (transitioning from `None` to `Some(id)`), sends a success message, and permanently yields control to the external `handle_user_session` function, breaking the loop. In case of invalid credentials or DB errors, it returns an error JSON message.
  - **register**: inserts the new user into the database via `users_repo` and returns a confirmation message, leaving the client inside the loop to allow immediate login.

- **Removal and Cleanup**: when the user session ends or if the client disconnects, execution exits the loop and final cleanup is performed, removing the `socket_id` from the global active sockets map in memory and logging the connection closure.

### User Session Handler

The user session handler module manages the complete lifecycle of an active connection after the client successfully completes authentication. Through the `handle_user_session` function, the server monitors connection status in real time and periodically receives telemetry data sent by the device.

The module uses the asynchronous macro `tokio::select!` to manage the following scenarios concurrently and without blocking:

#### 1. Ping/Pong and Timeout Management

To verify connection stability and promptly catch silent disconnections (e.g. driving through tunnels or sudden signal loss), the module implements a heartbeat mechanism:

- An asynchronous timer (`ping_interval`) is configured to fire every 10 seconds. It uses the `MissedTickBehavior::Skip` policy to prevent server slowdowns from accumulating backed-up ticks and firing them all at once.
- On every timer tick, if the client has not yet responded to the previous Ping (`waiting_for_pong` flag equals `true`), the connection is considered unstable or dropped; the server logs the timeout via `G19::warn!` and exits the loop, closing the session.
- If the client was responsive, the server sends a new `Ping` frame via the `tx` channel and sets the `waiting_for_pong` flag to `true`. When the client responds with a `Pong` frame, the flag is reset.

#### 2. Packet Reception from Client

The server continuously listens on the WebSocket read channel (`ws_receiver`). Based on the type of incoming packet, it executes specific actions:

- **Control Frames (Pong and Close)**: upon receiving a `Pong` frame, it resets the timeout warning state. Upon receiving a `Close` message, it immediately terminates the session, logging the closure.
- **STOP Command**: within text messages, the server immediately checks if the text matches the string `"STOP"`. If so, it sends a confirmation message to the client, logs the operation, and exits the session loop to begin cleaning up in-memory data.
- **Coordinates Reception**: if the received text is a valid JSON matching the `Coordinates` struct, the module performs the following operations:
  - Validates the timestamp format `created_at` using the helper function `convert_sql_to_naive_datetime`.
  - Passes the geographical data to the `UserStateHandler` struct to compute the vehicle's current state (e.g. stationary or in motion).
  - Invokes the `insert_journey_waypoint` function of the respective repository to save latitude, longitude, timestamp, and calculated state in the database `journeys` table.
  - Resets the `waiting_for_pong` flag to `false`, since receiving valid telemetry data autonomously proves the client is active and connected.
- **Standard Text Messages**: if the text is JSON matching a generic message (`utils::message::Message`), the server extracts the text body (`body`) and records it in system logs. If the format is unrecognized, it sends a JSON response containing an error message.

### User State Handler

The user state handler module calculates and monitors in real time the movement state of a client (vehicle) during its active session. Periodically receiving geographic coordinates, the `UserStateHandler` struct determines whether the vehicle should be considered moving (`false`) or stopped/paused (`true`), based on comparison with previously sent data.

The data structure maintains three key fields in memory to track history:

- **previous_coordinates**: stores the last set of coordinates (`Coordinates`) previously received and validated by the server.
- **sum_continuous_stop_time**: a counter (in seconds) accumulating continuous time the vehicle stayed stationary at the exact same location. It is reset as soon as the vehicle resumes movement.
- **first_eq_coordinates**: a boolean flag used to determine if the client has never moved since the start of the session (initially set to `true`).

The module also defines two internal constants to regulate logic:
- `DEFAULT_USER_STATE` (set to `true`): indicates initial default state (stationary vehicle).
- `MOTIONLESS_THRESHOLD_SECS` (set to 180 seconds / 3 minutes): minimum time threshold beyond which the vehicle is officially considered stopped if coordinates do not change.

#### State Calculation Logic (calculate_user_state Method)

Whenever the server receives a new geographic point, it invokes the `calculate_user_state` function, which determines the vehicle's state following this three-scenario logic:

1. **First point of the session**: if no previous coordinates exist (`previous_coordinates` is `None`), the system assigns the initial default state (`true`, i.e., stationary) and saves current coordinates as the next reference point.

2. **Coordinates identical to previous**: if current latitude and longitude equal the last transmission, the vehicle has not changed position.
   - If the vehicle has never moved since the start of the session (`first_eq_coordinates` is still `true`), the state remains stably set to stationary (`true`).
   - If it had already moved previously, the module calculates the time difference between the two transmissions (calling an internal function) and adds it to the `sum_continuous_stop_time` counter. Only when this counter reaches or exceeds the threshold of **3 minutes** (180 seconds) does the state officially transition to stationary (`true`); prior to exceeding the threshold, it is still considered moving.

3. **Different coordinates (Movement)**: if latitude or longitude change relative to the last transmission, movement is detected immediately. The `first_eq_coordinates` flag is set to `false`, the stop time counter is reset to `0`, and returned state is moving (`false`).

At the end of each calculation, the struct updates `previous_coordinates` with the newly processed coordinates to prepare for the next comparison.

### Server Messaging

The server messaging module implements forwarding logic for text messages typed by the administrator via the CLI to connected clients. It exposes two main async functions that interact directly with the socket map of `ConnectionManager` to identify active and authenticated recipients.

Messages sent to clients are encapsulated in a standard `Message` struct (containing the `body` field) and serialized to JSON format before being transmitted as WebSocket text.

The module provides the following sending functions:

- **Single Send (send Function)**: delivers a private message to a specific user.
  - Verifies that the `<user_id>` argument is a valid integer and joins remaining parameters to form the message body.
  - Acquires the active socket map in read mode (`manager.sockets.read().await`) and iterates to find the client whose `user_id` matches the requested recipient.
  - If found, inserts the serialized message into its internal `tx` transmission channel. If the channel rejects the message (because it closed), a warning is printed; if the user is missing or not yet authenticated, the administrator receives a user not found notification.

- **Global Send (broadcast Function)**: transmits a message simultaneously to all connected devices.
  - Joins parameters passed after the command to compose message text and serializes it to JSON.
  - Iterating over all channels in the global socket map, it selects exclusively those that completed authentication (where `user_id` `is_some()`).
  - Attempts sending on each respective `tx` channel, incrementing an internal counter. At the end of the loop, it prints a summary on the server console with the exact number of active users that successfully received the message.

### Database

This module generates the file at `src/server/database/database.sqlite` (if non-existent) and contains the following tables:

- users: contains `username` and `password` used to manage user registration/login (vehicles in our case). Passwords are saved hashed using random salt;
- journeys: contains `user_id`, `lat`, `lon`, `is_stopped`, `created_at` used to save individual geographic positions periodically sent by clients. `created_at` is used as a timestamp for statistics calculations.

### Journeys Repository

The journeys repository module manages persistent read and write operations on the `journeys` table of the SQLite database, using the `sqlx` async toolkit. The `JourneysRepository` struct encapsulates a connection pool (`Pool<Sqlite>`) and provides functions to save client telemetry and extract historical data needed for statistical calculations.

Key capabilities provided by the repository:

- **insert_journey_waypoint**: saves a single tracking point (waypoint) sent by a client to the database. After SQL insert query execution, a debug event is logged via `G19::debug!` to trace stored data.

- **get_full_journey_by_user_id**: retrieves the complete list of all geographic points recorded for a given user since the start of their trips.

- **get_journey_by_user_id_between_times**: extracts geographical points for a user within a specific time window.

- **get_total_pauses_by_user_id**: calculates total pause duration in seconds for a user within a given time interval.

### Users Repository

This module handles communication with the database `users` table, interfacing with the external `Authenticator` module for cryptographic operations. The `UsersRepository` struct contains a connection pool (`Pool<Sqlite>`) and defines the `AuthenticationStatus` enum representing login attempt outcomes.

The `AuthenticationStatus` enum has two variants:
- `Success(i64)`: indicates authentication succeeded and contains the `user_id` inserted into the table;
- `InvalidCredentials`: indicates authentication failed.

Main functions:
- `insert_user`: saves a new user (vehicle) in the table;
- `validate_user_credentials`: checks if provided credentials match those in the table, using `Authenticator` for cryptographic handling.
- `get_password_and_id_by_username`: returns password and ID of a user, searching by username.

### Authenticator

This module manages all user registration and login operations for the server.

Key features:
- `generate_salt`: generates a random value (*salt*), used when hashing passwords so users with identical passwords receive distinct hashes;
- `encrypt_password`: converts plaintext input password and returns a string containing calculated hash;
- `verify_password`: compares a plaintext password entered during login against the hash previously retrieved from the database.

### Statistics

This module is tightly coupled with `src/server/utils`, where purely mathematical calculations (such as distance in km and time differences) are implemented.

Statistics defines the time interval over which statistics are computed and uses `journeys_repository` for DB queries with functions defined in `utils`.

To compute metrics, the module relies on three main components:

- `RequiredTimeFrame`: an enum defining time windows. Variants are `CurrentDay`, `CurrentWeek`, and `CurrentMonth`;
- `TimeRange`: a utility struct containing two strings, `start` and `end`, representing timestamps formatted according to the exact SQLite database standard (`%Y-%m-%d %H:%M:%S`) for comparisons.

#### Main Features

The module exposes the following methods for statistics management:

- `convert_timeframe_to_range`: identifies current local time and, according to the `RequiredTimeFrame` variant, converts it to `start` and `end` strings returned inside `TimeRange`;
- `get_all`: wrapper calling statistics calculation functions and printing results to screen, specifically:
  - total kilometers traveled (kilometers);
  - average speed (km/h);
  - total movement hours (hours);
  - total pause hours (hours).
  
  Once query results are obtained and values computed via `utils` functions, the function formats data rounded to three decimal places (`{:.3}`) and prints the complete synthetic vehicle report to stdout.

#### Private Methods

Functions called by the `get_all` wrapper share a common pattern: converting the time interval into a `TimeRange` and querying the database to fetch results:
- `get_traveled_distance_by_user_id`: retrieves total kilometers traveled by the vehicle. If at least 2 points exist, `calculate_total_distance_of_journeys` in `utils` is called to get final distance; otherwise returns `0.0` as movement cannot be determined;

- `get_average_speed_by_user_id`: calculates average speed in km/h using formula **total_distance / total_hours**. If points <= 2 or total journey hours calculated by `calculate_total_hours_of_journeys` equals `0.0`, returns `0.0` to avoid division by zero. Otherwise divides total distance by total journey time;

- `get_full_movement_duration_by_user_id`: calculates total hours spent by vehicle between recorded points. If at least 2 points exist, delegates calculation to `calculate_total_hours_of_journeys` in `utils` and returns result;

- `get_pauses_hours_by_user_id`: retrieves total pause duration of vehicle and converts it to hours. Mathematical calculation is delegated to database query.

### Logger

The server includes a logging module fulfilling two functional requirements:
- structured logging of internal events;
- structured logging of process performance during execution.

The logger entity is designed as a singleton: initialization method `pub async fn init(path: impl AsRef<Path>, min_level: LogLevel, perf_interval: Duration) -> Result<(), InitError>` creates:
- a non-blocking `mpsc` channel;
- a performance collection task;
- a file writer task.

Additionally, minimum log level and log file path are specified. The system manages a single log file. File writing is structured in `json` format: each log line contains defined fields suitable for tools like OpenObserve and Grafana Loki. The `json` structure is represented by `LogLine`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub time: String,
    pub level: LogLevel,
    pub module: LogModule,
    pub event: String,
    pub message: String,
}
```

The module exposes 4 macros externally for inserting log lines: `debug!`, `info!`, `warn!`, and `error!`. Callers write to the channel asynchronously without blocking execution. The logger also exposes `flush`, forcing the writer task to finish inserting lines to file before shutdown.
