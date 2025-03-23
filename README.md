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

## Communication Method

The manager actor uses a channel-based communication protocol for real-time progress updates and operation coordination:

For real-time progress updates, use the channel-based communication:

1. Open a channel to the manager actor:
```bash
theater channel-open manager --data '{"client_type":"frontend","version":"1.0"}'
```

2. Send commands through the channel:
```bash
# Build command
theater channel open <channel-id> '{"BuildActor":null}'

# Change request
channel> send <channel-id> '{"ChangeRequest":{"description":"Add new feature"}}'

# Start actor
channel> send <channel-id> '{"StartActor":null}'

# Stop actor
channel> send <channel-id> '{"StopActor":null}'

# Get status
channel> send <channel-id> '{"GetStatus":null}'
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


## Project Structure

The manager actor is organized into the following modules:

- `state`: Defines the state structure and operations state tracking
- `messaging`: Contains message formats for frontend communication
- `operations`: Implements command handlers for build and change operations
- `lib.rs`: Main actor implementation with channel handling logic

## Dependencies

- Build Actor: Used for compiling code to WebAssembly
- Programmer Actor: Used for code generation and changes (uses Claude API)
