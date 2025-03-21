# Manager Actor Channel Communication Change Request

## Overview

This change request outlines the modifications needed to implement channel-based communication in the manager actor. The manager actor will serve as a central hub that:

1. Accepts channel connections from the frontend
2. Establishes channels with both the build actor and programmer actor
3. Forwards messages between the frontend and appropriate actors

## Architectural Changes

### 1. Channel Management

**Current State**: The manager actor currently uses request/response communication with child actors and does not maintain persistent connections with the frontend.

**Target State**: The manager actor will:
- Accept and maintain a persistent channel with the frontend client
- Create and manage channels to the build and programmer actors for long-running operations
- Act as a message router between the frontend and other actors

### 2. State Management Updates

The manager actor needs to track:
- Frontend channel ID
- Map of operation IDs to channels for active actor communications
- Current status of all operations

```rust
struct AppState {
    // Existing fields
    child_id: Option<String>,
    programmer_actor_id: String,
    runtime_content_fs_actor_id: String,
    build_store_id: String,
    
    // New fields
    frontend_channel_id: Option<String>,
    actor_channels: HashMap<String, String>, // operation_id -> channel_id
    active_operations: HashMap<String, OperationState>,
}

struct OperationState {
    operation_id: String,
    operation_type: OperationType,
    actor_id: String,
    channel_id: Option<String>,
    status: OperationStatus,
    start_time: u64,
    end_time: Option<u64>,
}

enum OperationType {
    Build,
    Change,
    Start,
    Stop,
}

enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}
```

### 3. Channel Handler Implementation

Implement the following channel handlers in the manager actor:

#### 3.1. Channel Open Handler
- Accept connections from the frontend client
- Store the channel ID for future communication

#### 3.2. Channel Message Handler
- Process incoming messages from the frontend
- Forward actor messages to the frontend
- Track operation state changes

#### 3.3. Channel Close Handler
- Clean up resources when channels are closed
- Handle unexpected disconnections

### 4. Message Flow

#### Frontend to Manager
```
Frontend --[FrontendCommand]--> Manager
```

Frontend commands include:
- `StartActor` - Start the child actor
- `StopActor` - Stop the child actor 
- `BuildActor` - Build the WebAssembly actor
- `ChangeRequest` - Submit a code change request
- `GetStatus` - Get the current system status
- `Disconnect` - Close the connection

#### Manager to Frontend
```
Manager --[FrontendMessage]--> Frontend
```

Frontend messages include:
- `Status` - Current status of the system
- `OperationStarted` - Operation has been initiated
- `OperationProgress` - Updates on operation progress 
- `OperationCompleted` - Operation has finished
- `BuildEvent` - Events from the build actor
- `ProgrammerEvent` - Events from the programmer actor
- `Log` - Log messages
- `Error` - Error messages

#### Manager to Build Actor
```
Manager --[channel.open]--> Build Actor
Manager --[build.start]--> Build Actor
Build Actor --[build.progress]--> Manager
Build Actor --[build.complete]--> Manager
```

#### Manager to Programmer Actor
```
Manager --[channel.open]--> Programmer Actor
Manager --[change.request]--> Programmer Actor
Programmer Actor --[progress/log/tool_use]--> Manager
Programmer Actor --[task.complete]--> Manager
```

### 5. Operation Workflow

#### Build Operation
1. Frontend sends `BuildActor` command to manager
2. Manager creates an operation ID and opens channel to build actor
3. Manager forwards the build command through the channel
4. Build actor streams progress updates through the channel
5. Manager forwards updates to the frontend
6. Build actor sends completion message
7. Manager updates operation status and notifies frontend

#### Change Operation
1. Frontend sends `ChangeRequest` command to manager
2. Manager creates an operation ID and opens channel to programmer actor
3. Manager forwards the change request through the channel
4. Programmer actor streams progress, tool use, and response events
5. Manager forwards events to the frontend
6. Programmer actor sends completion message
7. Manager updates operation status and notifies frontend

## Implementation Steps

1. Update the `AppState` structure to include channel tracking fields
2. Implement the channel open handler for frontend connections
3. Create message type definitions for frontend commands and responses
4. Implement the channel message handler for frontend commands
5. Implement operation tracking and management
6. Add channel establishment to build and programmer actors
7. Implement message forwarding between actors and frontend
8. Add error handling and recovery mechanisms
9. Update existing request handlers to use channels for long operations

## Testing Requirements

1. Test frontend channel establishment
2. Test streaming build updates
3. Test streaming programmer updates
4. Test error conditions and recovery
5. Test channel closure handling
6. Test concurrent operations

## Security Considerations

1. Validate all incoming messages before processing
2. Ensure proper channel cleanup when operations complete
3. Handle authentication for frontend connections (future enhancement)
4. Protect operation IDs from tampering

## Acceptance Criteria

1. The manager actor accepts and maintains a channel connection with the frontend
2. Build and change operations use channels for real-time updates
3. All messages are properly routed between the frontend and actors
4. The system maintains proper state for all operations
5. Clean shutdown and cleanup when channels are closed
