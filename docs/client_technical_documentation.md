# Client Technical Documentation

This document explains the architecture and design of the Georust client module.

Modules:
- `main.rs`: central orchestrator
- `config.rs`: configuration manager
- `auth.rs`: authentication towards the server
- `coord_gen.rs`: generation of coordinates to send
- `console.rs`: interactive console manager

Crates used:
- `tokio`
- `tokio tungstenite`
- `clap`
- `serde`

## Execution Flow

### Initialization and Connection

As soon as the client binary is executed, the following configuration values are read and set:
- `coord_file_path`
- `tick_interval_millis`
- `client_username`
- `client_password`
- `server_url`

These values are initially read from the `client_config.json` file and command-line parameters (using the `clap` crate), which take priority. The `generator` object is then initialized, allowing `main` to sequentially obtain the coordinates to send. After completing this operation, the client connects to the server, receiving a read/write stream and the response.

### Authentication

Before being able to exchange messages with the server, the client must authenticate: the relevant functions are contained in the `auth.rs` module which exposes a single function:

```rust
pub async fn authenticate(
    cfg: &Config,
    ws_write: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    ws_read: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
) -> Result<i64, Box<dyn std::error::Error>>
```

Which accepts as input the configuration and the read/write streams obtained during the connection phase.

### Sending Messages to the Server

Once authenticated, the client is ready to send messages to the server as JSON structures shared between the binaries. Two new asynchronous tasks, `writer_task` and `reader_task`, are spawned for writing to and reading from the WebSocket stream.

Coordinates are read from a file via the `coord_gen.rs` module, which exposes initialization and current coordinate retrieval functions:

```rust
pub fn init(path: &str) -> io::Result<Self>
pub fn get_next(&mut self) -> Option<Coordinates>
```

The `get_next` method, coupled with the `ticker` object, is used inside a `loop` to send the read coordinate at regular time intervals determined by the configuration field `cfg.tick_interval_millis`. The message is then passed to `writer_task`, which handles sending it to the server.

### Interactive Console

While the client process is running, an interactive console is available to the user, allowing them to send/receive text messages from the server, pause coordinate transmission, or terminate the process. This logic is handled by `console.rs`, an asynchronous task spawned by `main.rs` that uses clones of pre-existing channels for communication with the primary task (via `event_tx`) and the server (via `out_tx`):

```rust
pub async fn run(
    out_tx: mpsc::Sender<Message>,
    event_tx: mpsc::Sender<ConsoleEvent>
    )
```

The module is a simple parser that uses a `BufReader` to read `tokio::io::stdin()`, where the user types commands to be processed individually by `pub fn handle_event(event: ConsoleEvent, sending: &mut bool) -> LoopControl`. The return type `LoopControl` is a struct that declares whether the `main` loop should continue: it is used by the `exit` command to terminate the process.
