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

## Manager Actor API

You can interact with the manager actor using the following actions:

- `Start` - Start a child actor
- `Stop` - Stop a running child actor
- `Build` - Build the current code
- `Change` - Submit code changes to the programmer actor

## Example

```bash
# Send a build request to the manager actor
theater message manager '{"action":"Build"}'
```

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
