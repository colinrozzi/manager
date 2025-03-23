# Theater Studio CLI and Manager Actor Interface Guide

This document provides a comprehensive reference for the communication interface between the Theater Studio CLI and the Manager actor. It covers all message formats, workflows, and implementation requirements.

## 1. Communication Protocol Overview

The Theater Studio CLI communicates with the Manager actor using a channel-based communication protocol, enabling real-time updates and bidirectional messaging.

### Establishing Connection

```bash
# Open a channel to the manager actor
theater channel-open manager --data '{"client_type":"frontend","version":"1.0"}'
```

The Manager actor expects an initial connection message with the client type specified as "frontend". Upon successful connection, the Manager will respond with a welcome message and initial status.

## 2. Command Structure

The CLI sends commands to the Manager actor using the `FrontendCommand` enum structure serialized as JSON:

```json
// Available commands:
{"StartActor": null}
{"StopActor": null}
{"BuildActor": null}
{"ChangeRequest": {"description": "Description of the change"}}
{"GetStatus": null}
{"Disconnect": null}
```

These commands are sent through the channel using:

```bash
theater channel-send <channel-id> '<command-json>'
```

## 3. Response Structure

The Manager actor sends messages back to the CLI using the `FrontendMessage` enum structure:

### Status Messages
```json
{
  "Status": {
    "child_running": true|false,
    "active_operations": [
      {
        "operation_id": "uuid-string",
        "operation_type": "Build|Change|Start|Stop",
        "status": "Pending|InProgress|Completed|Failed"
      }
    ]
  }
}
```

### Operation Updates
```json
{
  "OperationStarted": {
    "operation_id": "uuid-string",
    "operation_type": "Build|Change|Start|Stop",
    "description": "Description of the operation"
  }
}

{
  "OperationProgress": {
    "operation_id": "uuid-string",
    "description": "Current progress detail",
    "percent_complete": 50.0
  }
}

{
  "OperationCompleted": {
    "operation_id": "uuid-string",
    "success": true|false,
    "message": "Completion message"
  }
}
```

### Actor Events
```json
{
  "BuildEvent": {
    "operation_id": "uuid-string",
    "event_type": "Log|Progress|CommandStarted|CommandOutput|FileExtracted|BuildComplete",
    "message": "Event description",
    "details": { /* JSON object with event-specific details */ }
  }
}

{
  "ProgrammerEvent": {
    "operation_id": "uuid-string",
    "event_type": "event-type-string",
    "message": "Event description",
    "details": { /* JSON object with event-specific details */ }
  }
}
```

### Log and Error Messages
```json
{
  "Log": {
    "level": "info|warn|error|debug",
    "message": "Log message"
  }
}

{
  "Error": {
    "code": "error-code-string",
    "message": "Error description"
  }
}
```

## 4. Operation Workflows

### Build Workflow

1. Send build command:
   ```json
   {"BuildActor": null}
   ```

2. Receive operation started:
   ```json
   {
     "OperationStarted": {
       "operation_id": "uuid-string",
       "operation_type": "Build",
       "description": "Building WebAssembly actor"
     }
   }
   ```

3. Receive build progress events:
   ```json
   {
     "BuildEvent": {
       "operation_id": "uuid-string",
       "event_type": "Progress",
       "message": "Compiling...",
       "details": {
         "status": "compiling",
         "percent_complete": 30.0
       }
     }
   }
   ```

4. Receive completion:
   ```json
   {
     "OperationCompleted": {
       "operation_id": "uuid-string",
       "success": true,
       "message": "Build completed successfully"
     }
   }
   ```

### Change Workflow

1. Send change request:
   ```json
   {"ChangeRequest": {"description": "Add new endpoint for user profile"}}
   ```

2. Receive operation started:
   ```json
   {
     "OperationStarted": {
       "operation_id": "uuid-string",
       "operation_type": "Change",
       "description": "Processing code change: Add new endpoint for user profile"
     }
   }
   ```

3. Receive programmer events during processing:
   ```json
   {
     "ProgrammerEvent": {
       "operation_id": "uuid-string",
       "event_type": "ThinkingStart",
       "message": "Analyzing code structure",
       "details": {
         "thinking": true
       }
     }
   }
   ```

4. Receive completion when the change is finished:
   ```json
   {
     "OperationCompleted": {
       "operation_id": "uuid-string",
       "success": true,
       "message": "Code change successfully applied"
     }
   }
   ```

### Start Actor Workflow

1. Send start actor command:
   ```json
   {"StartActor": null}
   ```

2. Receive operation updates as the actor starts.

3. Receive completion when the actor is running.

### Stop Actor Workflow

1. Send stop actor command:
   ```json
   {"StopActor": null}
   ```

2. Receive operation updates as the actor stops.

3. Receive completion when the actor has stopped.

### Status Request Workflow

1. Send status request:
   ```json
   {"GetStatus": null}
   ```

2. Receive status message with current system state.

## 5. Event Handling Requirements

The CLI should implement the following event handling capabilities:

1. **Handle Asynchronous Events**: Process messages as they arrive without blocking the user interface
   
2. **Track Operation IDs**: Maintain a mapping of active operations to display status updates correctly
   
3. **Display Progress**: Show real-time progress indicators for long-running operations
   
4. **Log Management**: Filter and display log messages with appropriate formatting based on level

## 6. Error Handling

The Manager actor can send error messages in two ways:

1. As an `Error` message:
   ```json
   {
     "Error": {
       "code": "command_error",
       "message": "Failed to start build: No source code found"
     }
   }
   ```

2. As an `OperationCompleted` message with `success: false`:
   ```json
   {
     "OperationCompleted": {
       "operation_id": "uuid-string",
       "success": false, 
       "message": "Build failed due to compilation errors"
     }
   }
   ```

The CLI should handle both types of errors appropriately, displaying clear error messages to the user and offering remediation steps when possible.

## 7. Channel Management

The CLI should implement proper channel management:

1. **Keep-Alive**: The channel connection should be maintained throughout the CLI session
   
2. **Reconnection**: If the channel is closed unexpectedly, the CLI should attempt to reopen it
   
3. **Graceful Shutdown**: Send a `Disconnect` command before closing the channel

4. **Channel Timeout Handling**: Implement timeout detection and recovery

## 8. Build Event Details Structure

Build events come with detailed information in the "details" field:

```json
// For Log events
{
  "level": "info|warn|error|debug"
}

// For Progress events
{
  "status": "status-string",
  "percent_complete": 75.0
}

// For CommandStarted events
{
  "command": "cargo",
  "args": ["build", "--target", "wasm32-unknown-unknown"]
}

// For CommandOutput events
{
  "stdout": "stdout content",
  "stderr": "stderr content"
}

// For FileExtracted events
{
  "path": "/path/to/file",
  "size": 1024
}

// For BuildComplete events
{
  "success": true|false,
  "wasm_path": "/path/to/actor.wasm",
  "wasm_hash": "hash-string",
  "error": "error message if success is false"
}
```

## 9. Programmer Event Details Structure

Programmer events (related to code changes) include details such as:

```json
// For thinking/processing events
{
  "thinking": true,
  "progress": 50
}

// For code generation events
{
  "file": "src/lib.rs",
  "changes": [
    {
      "type": "add|modify|delete",
      "line_start": 10,
      "line_end": 15,
      "content": "new content"
    }
  ]
}

// For completion events
{
  "files_changed": ["src/lib.rs", "src/api.rs"],
  "summary": "Added new user endpoint and tests"
}
```

## 10. Implementation Recommendations

For optimal user experience, the CLI should:

1. **Response Parsing**: Implement a robust JSON parser that can handle all message types
   
2. **Progress Display**: Use a spinner or progress bar for long-running operations
   
3. **Command Validation**: Validate user commands before sending them to the Manager
   
4. **State Tracking**: Maintain a local state model to track operations and actor status
   
5. **Error Recovery**: Implement graceful recovery from connection issues or failed operations

6. **Colorized Output**: Use color coding for different log levels and message types

7. **Command History**: Maintain a history of commands for easy reuse

8. **Tab Completion**: Implement tab completion for commonly used commands

## 11. Example Command Sequence

A typical development workflow might include:

```
1. User: start
   CLI: Opens channel to manager
   Manager: Sends welcome message and initial status

2. User: change "Add user profile API"
   CLI: Sends ChangeRequest command
   Manager: Sends operation started
   Manager: Sends programmer events as processing occurs
   Manager: Sends operation completed when done

3. User: build
   CLI: Sends BuildActor command
   Manager: Sends operation started and build events
   Manager: Sends operation completed when build finishes

4. User: start-actor
   CLI: Sends StartActor command
   Manager: Starts the actor and confirms

5. User: http GET /api/users
   CLI: Sends HTTP request to the running actor
   CLI: Displays response

6. User: stop-actor
   CLI: Sends StopActor command
   Manager: Stops the actor and confirms
```

## 12. Message Format Schema

For validation purposes, here is a JSON Schema representation of the message formats:

### Frontend Command Schema
```json
{
  "type": "object",
  "oneOf": [
    { "required": ["StartActor"], "properties": { "StartActor": { "type": "null" } } },
    { "required": ["StopActor"], "properties": { "StopActor": { "type": "null" } } },
    { "required": ["BuildActor"], "properties": { "BuildActor": { "type": "null" } } },
    { "required": ["ChangeRequest"], "properties": { "ChangeRequest": { "type": "object", "required": ["description"], "properties": { "description": { "type": "string" } } } } },
    { "required": ["GetStatus"], "properties": { "GetStatus": { "type": "null" } } },
    { "required": ["Disconnect"], "properties": { "Disconnect": { "type": "null" } } }
  ]
}
```

### Frontend Message Schema
```json
{
  "type": "object",
  "oneOf": [
    {
      "required": ["Status"],
      "properties": {
        "Status": {
          "type": "object",
          "required": ["child_running", "active_operations"],
          "properties": {
            "child_running": { "type": "boolean" },
            "active_operations": {
              "type": "array",
              "items": {
                "type": "object",
                "required": ["operation_id", "operation_type", "status"],
                "properties": {
                  "operation_id": { "type": "string" },
                  "operation_type": { "type": "string", "enum": ["Build", "Change", "Start", "Stop"] },
                  "status": { "type": "string", "enum": ["Pending", "InProgress", "Completed", "Failed"] }
                }
              }
            }
          }
        }
      }
    },
    // Additional message types would follow a similar pattern
  ]
}
```

## 13. Troubleshooting

Common issues and their solutions:

1. **Channel Connection Failures**
   - Ensure the Manager actor is running
   - Check that the correct channel-open command is being used
   - Verify network connectivity if running distributed

2. **Command Not Recognized**
   - Ensure JSON formatting is correct
   - Verify command spelling and capitalization
   - Check for extraneous whitespace in the command

3. **Missing Operation Updates**
   - Check that the channel is still open
   - Verify the operation ID is being tracked correctly
   - Ensure no messages are being dropped

4. **Build Failures**
   - Examine BuildEvent messages for compilation errors
   - Check that the source code is valid
   - Verify build environment configuration

This guide should be used as the definitive reference for implementing the Theater Studio CLI interface.
