# Server Usage Documentation

This guide outlines the features, startup parameters, and interactive CLI commands available for the **Georust** server.

## Main Features

Once started, the server performs several operations in parallel:

- **Connected Client Management**:
  - Vehicle authentication and registration.
  - Reception and persistence of GPS coordinates sent by clients.
  - Calculation and update of current vehicle state (moving, stopped, speed calculation).
  - Reception of text messages from clients.
- **Interactive Console (CLI)**:
  - Processing commands typed by the administrator on the server terminal.
  - Calculation and display of route statistics.
  - Communication tools (broadcast messages or direct messages to a specific vehicle).
- **Monitoring and Logging**:
  - Performance logging (CPU) and running task tracking to facilitate debugging.

## Starting the Server

Always start the server before any clients:

```bash
cargo run --bin server --release -- [--with-init]
```
where `--with-init` is an optional parameter that allows resetting the database tables.

## Interactive Console

Once the server is started, it launches a background task that listens for terminal inputs. Supported commands are listed below:

> [!IMPORTANT]
> Commands requiring the `<user_id>` parameter require the **internal identifier** registered in the database for that user, not the plain username (e.g., typing `client1` is not enough). To retrieve the exact vehicle ID to use in the command, check the database records directly.

| Command | Description |
| :--- | :--- |
| `statistics <user_id> [DAY\|WEEK\|MONTH]` | Calculates and displays statistics (distance traveled, average speed, movement duration, and pause duration) for the specified vehicle.<br><br>*Note: if the time period is omitted, `DAY` is used as default.* |
| `send <user_id> <message>` | Sends a private text message to a specific vehicle. |
| `broadcast <message>` | Sends a text message to **all** vehicles currently connected to the server. |
| `help` | Displays the complete list of supported commands with brief descriptions. |
