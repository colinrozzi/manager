# Theater Manager Actor & Studio CLI

The manager actor is a core component of the Theater actor system that supervises other actors and provides orchestration capabilities.

## Building

To build the actor:

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Running

To run the actor with Theater:

```bash
theater start manifest.toml
```

## Features

The manager actor provides:

- Actor supervision (start/stop child actors)
- Build coordination (via build actor)
- Integration with the programmer actor for code changes
- State management and persistence
- Real-time progress streaming using channels

## Communication Methods

The manager actor supports two communication methods:

### 1. Request/Response API

You can interact with the manager actor using the following actions:

- `Start` - Start a child actor
- `Stop` - Stop a running child actor
- `Build` - Build the current code
- `Change` - Submit code changes to the programmer actor

Example:
```bash
# Send a build request to the manager actor
theater message manager '{"action":"Build"}'
```

### 2. Channel-Based Communication (Recommended)

For real-time progress updates, use the channel-based communication:

1. Open a channel to the manager actor:
```bash
theater channel-open manager --data '{"client_type":"frontend","version":"1.0"}'
```

2. Send commands through the channel:
```bash
# Build command
theater channel-send <channel-id> '{"BuildActor":null}'

# Change request
theater channel-send <channel-id> '{"ChangeRequest":{"description":"Add new feature"}}'

# Start actor
theater channel-send <channel-id> '{"StartActor":null}'

# Stop actor
theater channel-send <channel-id> '{"StopActor":null}'

# Get status
theater channel-send <channel-id> '{"GetStatus":null}'
```

3. Receive progress updates in real-time:
```bash
theater channel-recv <channel-id>
```

## Channel Protocol

The channel-based communication provides detailed progress updates:

### Frontend Commands:
- `StartActor` - Start the child actor
- `StopActor` - Stop the running child actor 
- `BuildActor` - Build the WebAssembly actor
- `ChangeRequest { description: String }` - Submit a code change request
- `GetStatus` - Get the current system status
- `Disconnect` - Close the connection

### Manager Responses:
- `Status` - Current status of the system
- `OperationStarted` - Operation has been initiated
- `OperationProgress` - Updates on operation progress 
- `OperationCompleted` - Operation has finished
- `BuildEvent` - Events from the build actor
- `ProgrammerEvent` - Events from the programmer actor
- `Log` - Log messages
- `Error` - Error messages

## Theater Studio CLI

This package includes a REPL-style CLI for interacting with the Theater actor system.

### Building the CLI

```bash
cargo build --bin theater-studio
```

### Running the CLI

```bash
cargo run --bin theater-studio
```

### Available Commands

#### Session Commands
- `start` - Start a new development session
- `stop` - Stop the current development session
- `status` - Show the status of the current session

#### Code Commands
- `change <desc>` - Submit code changes with description
- `build` - Build the current code into a WebAssembly actor

#### Actor Commands
- `start-actor` - Start the built actor
- `stop-actor` - Stop the running actor
- `restart-actor` - Restart the running actor
- `logs` - Show actor logs

#### Interaction Commands
- `message <content>` - Send a message to the running actor
- `state` - Display the actor's current state
- `http <method> <path> [data]` - Send HTTP request to actor

#### Utility Commands
- `help` - Display help information
- `clear` - Clear the terminal screen
- `exit` - Exit the Theater Studio CLI

### Example Workflow

```
# Starting a session
theater> start

# Making code changes
theater> change "Add a new endpoint for user data"

# Building the actor
theater> build

# Starting the actor
theater> start-actor

# Sending an HTTP request
theater> http GET /api/users
```

## Project Structure

The manager actor is organized into the following modules:

- `state`: Defines the state structure and operations state tracking
- `messaging`: Contains message formats for frontend communication
- `operations`: Implements command handlers for build and change operations
- `lib.rs`: Main actor implementation with channel handling logic

## Dependencies

- Build Actor: Used for compiling code to WebAssembly
- Programmer Actor: Used for code generation and changes (uses Claude API)
