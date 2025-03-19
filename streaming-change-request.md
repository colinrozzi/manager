# Change Request: Manager Actor Channel Streaming Implementation

## Overview

This change request outlines the modifications needed for the manager actor to support channel-based streaming of events during operations. The manager will serve as the central hub for event streams, coordinating communication between clients, the programmer actor, the build actor, and child actors.

## Implementation Details

### 1. State Structure Updates

Update the `AppState` struct in `src/lib.rs` to include channel management capabilities:

```rust
// Add to AppState struct
struct AppState {
    // Existing fields
    child_id: Option<String>,
    programmer_actor_id: String,
    runtime_content_fs_actor_id: String,
    build_store_id: String,
    
    // New fields for channel management
    active_channels: HashMap<String, ChannelInfo>,
    operation_channels: HashMap<String, Vec<String>>, // Maps operation_id to channel_ids
    client_channels: HashMap<String, String>, // Maps client_id to channel_id
}

// New structure to track channel information
struct ChannelInfo {
    channel_id: String,
    channel_type: ChannelType,
    target_actor_id: String,
    operation_id: String,
    client_id: Option<String>,
    created_at: u64,
    last_activity: u64,
}

enum ChannelType {
    ClientStream,     // Channel to a client
    ProgrammerStream, // Channel to programmer actor
    BuildStream,      // Channel to build actor
    ChildActorStream, // Channel to a child actor
}
```

### 2. Channel Management Functions

Add functions to establish and manage channels:

```rust
// Open a channel to a client actor (e.g., CLI)
fn open_client_channel(&mut self, client_id: &str, operation_id: &str) -> Result<String, String> {
    let initial_message = json!({
        "event_type": "operation.started",
        "source": "manager",
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "sequence": 0,
        "operation_id": operation_id,
        "content": {
            "operation_type": "channel_established",
            "message": "Channel established for streaming updates"
        }
    });
    
    let message_bytes = serde_json::to_vec(&initial_message).map_err(|e| e.to_string())?;
    
    // Open channel to client
    let channel_id = ntwk_theater_message_server_host_open_channel(client_id, &message_bytes)?;
    
    // Store channel info
    let channel_info = ChannelInfo {
        channel_id: channel_id.clone(),
        channel_type: ChannelType::ClientStream,
        target_actor_id: client_id.to_string(),
        operation_id: operation_id.to_string(),
        client_id: Some(client_id.to_string()),
        created_at: chrono::Utc::now().timestamp_millis(),
        last_activity: chrono::Utc::now().timestamp_millis(),
    };
    
    self.active_channels.insert(channel_id.clone(), channel_info);
    self.client_channels.insert(client_id.to_string(), channel_id.clone());
    
    // Add to operation channels
    self.operation_channels
        .entry(operation_id.to_string())
        .or_insert_with(Vec::new)
        .push(channel_id.clone());
    
    Ok(channel_id)
}

// Open a channel to the programmer actor
fn open_programmer_channel(&mut self, operation_id: &str) -> Result<String, String> {
    let initial_message = json!({
        "event_type": "operation.started",
        "source": "manager",
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "sequence": 0,
        "operation_id": operation_id,
        "content": {
            "operation_type": "code_change",
            "stream_type": "events"
        }
    });
    
    let message_bytes = serde_json::to_vec(&initial_message).map_err(|e| e.to_string())?;
    
    // Open channel to programmer
    let channel_id = ntwk_theater_message_server_host_open_channel(
        &self.programmer_actor_id, 
        &message_bytes
    )?;
    
    // Store channel info
    let channel_info = ChannelInfo {
        channel_id: channel_id.clone(),
        channel_type: ChannelType::ProgrammerStream,
        target_actor_id: self.programmer_actor_id.clone(),
        operation_id: operation_id.to_string(),
        client_id: None,
        created_at: chrono::Utc::now().timestamp_millis(),
        last_activity: chrono::Utc::now().timestamp_millis(),
    };
    
    self.active_channels.insert(channel_id.clone(), channel_info);
    
    // Add to operation channels
    self.operation_channels
        .entry(operation_id.to_string())
        .or_insert_with(Vec::new)
        .push(channel_id.clone());
    
    Ok(channel_id)
}

// Open a channel to the build actor
fn open_build_channel(&mut self, build_actor_id: &str, operation_id: &str) -> Result<String, String> {
    let initial_message = json!({
        "event_type": "operation.started",
        "source": "manager",
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "sequence": 0,
        "operation_id": operation_id,
        "content": {
            "operation_type": "build",
            "stream_type": "events"
        }
    });
    
    let message_bytes = serde_json::to_vec(&initial_message).map_err(|e| e.to_string())?;
    
    // Open channel to build actor
    let channel_id = ntwk_theater_message_server_host_open_channel(
        build_actor_id, 
        &message_bytes
    )?;
    
    // Store channel info
    let channel_info = ChannelInfo {
        channel_id: channel_id.clone(),
        channel_type: ChannelType::BuildStream,
        target_actor_id: build_actor_id.to_string(),
        operation_id: operation_id.to_string(),
        client_id: None,
        created_at: chrono::Utc::now().timestamp_millis(),
        last_activity: chrono::Utc::now().timestamp_millis(),
    };
    
    self.active_channels.insert(channel_id.clone(), channel_info);
    
    // Add to operation channels
    self.operation_channels
        .entry(operation_id.to_string())
        .or_insert_with(Vec::new)
        .push(channel_id.clone());
    
    Ok(channel_id)
}

// Open a channel to a child actor
fn open_child_actor_channel(&mut self, child_id: &str, operation_id: &str) -> Result<String, String> {
    let initial_message = json!({
        "event_type": "operation.started",
        "source": "manager",
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "sequence": 0,
        "operation_id": operation_id,
        "content": {
            "operation_type": "monitor",
            "stream_type": "events"
        }
    });
    
    let message_bytes = serde_json::to_vec(&initial_message).map_err(|e| e.to_string())?;
    
    // Open channel to child actor
    let channel_id = ntwk_theater_message_server_host_open_channel(
        child_id, 
        &message_bytes
    )?;
    
    // Store channel info
    let channel_info = ChannelInfo {
        channel_id: channel_id.clone(),
        channel_type: ChannelType::ChildActorStream,
        target_actor_id: child_id.to_string(),
        operation_id: operation_id.to_string(),
        client_id: None,
        created_at: chrono::Utc::now().timestamp_millis(),
        last_activity: chrono::Utc::now().timestamp_millis(),
    };
    
    self.active_channels.insert(channel_id.clone(), channel_info);
    
    // Add to operation channels
    self.operation_channels
        .entry(operation_id.to_string())
        .or_insert_with(Vec::new)
        .push(channel_id.clone());
    
    Ok(channel_id)
}
```

### 3. Event Routing Function

Add a function to route events between channels:

```rust
// Route an event from a source to all relevant targets
fn route_event(&mut self, source_channel_id: &str, event: &serde_json::Value) -> Result<(), String> {
    // Find the channel info for the source
    let operation_id = match self.active_channels.get(source_channel_id) {
        Some(info) => info.operation_id.clone(),
        None => return Err(format!("Channel not found: {}", source_channel_id)),
    };
    
    // Get all channels for this operation
    let channel_ids = match self.operation_channels.get(&operation_id) {
        Some(ids) => ids.clone(),
        None => return Err(format!("No channels found for operation: {}", operation_id)),
    };
    
    // Forward the event to all channels except the source
    for channel_id in channel_ids {
        if channel_id != source_channel_id {
            let channel_info = match self.active_channels.get(&channel_id) {
                Some(info) => info,
                None => continue, // Skip if channel info not found
            };
            
            // Serialize the event
            let event_bytes = serde_json::to_vec(event).map_err(|e| e.to_string())?;
            
            // Send on channel
            if let Err(e) = ntwk_theater_message_server_host_send_on_channel(&channel_id, &event_bytes) {
                log(&format!("Failed to route event to channel {}: {}", channel_id, e));
                // Don't fail the whole operation if one channel fails
            }
            
            // Update last activity
            if let Some(channel_info) = self.active_channels.get_mut(&channel_id) {
                channel_info.last_activity = chrono::Utc::now().timestamp_millis();
            }
        }
    }
    
    Ok(())
}
```

### 4. Modified Action Handlers

Update the action handlers to use channels for streaming updates:

```rust
// Update the Action enum to include client_id and operation_id
#[derive(Serialize, Deserialize)]
enum Action {
    Start { client_id: String, operation_id: String },
    Stop { client_id: String, operation_id: String },
    Build { client_id: String, operation_id: String },
    Change { description: String, client_id: String, operation_id: String },
    OpenChannel { client_id: String, operation_id: String },
}

// Modify the Change action handler
fn handle_request(
    state: Option<Vec<u8>>,
    params: (Vec<u8>,),
) -> Result<(Option<Vec<u8>>, (Vec<u8>,)), String> {
    // Existing code...
    
    let action: Action = serde_json::from_slice(&data).map_err(|e| e.to_string())?;

    let (response, state_updated) = match action {
        // Modify Change action to use channels
        Action::Change { description, client_id, operation_id } => {
            log(&format!("Received change request: {}", description));
            
            // Set up channels
            let client_channel = app_state.open_client_channel(&client_id, &operation_id)?;
            let programmer_channel = app_state.open_programmer_channel(&operation_id)?;
            
            // Send initial event to client
            let initial_event = json!({
                "event_type": "operation.started",
                "source": "manager",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "sequence": 1,
                "operation_id": operation_id,
                "content": {
                    "operation_type": "code_change",
                    "description": description,
                    "status": "starting"
                }
            });
            
            let initial_event_bytes = serde_json::to_vec(&initial_event).map_err(|e| e.to_string())?;
            ntwk_theater_message_server_host_send_on_channel(&client_channel, &initial_event_bytes)?;
            
            // Send request to programmer via channel
            let change_request = json!({
                "event_type": "change.request",
                "source": "manager",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "sequence": 0,
                "operation_id": operation_id,
                "content": {
                    "description": description
                }
            });
            
            let change_request_bytes = serde_json::to_vec(&change_request).map_err(|e| e.to_string())?;
            ntwk_theater_message_server_host_send_on_channel(&programmer_channel, &change_request_bytes)?;
            
            // Return immediate response to unblock the caller
            (json!({
                "status": "started", 
                "operation_id": operation_id,
                "channel_id": client_channel
            }).to_string().into_bytes(), true)
        },
        
        // Modify Build action to use channels
        Action::Build { client_id, operation_id } => {
            log(&format!("Received build request for operation: {}", operation_id));
            
            // Set up client channel
            let client_channel = app_state.open_client_channel(&client_id, &operation_id)?;
            
            // Get runtime content fs info
            let runtime_info_response = request(
                &app_state.runtime_content_fs_actor_id,
                &serde_json::to_vec(&json!({"action": "get-info", "params": []})).unwrap(),
            )
            .expect("Failed to get runtime info");
            
            log(&format!("Received runtime info: {}", String::from_utf8_lossy(&runtime_info_response)));
            
            // Spawn build actor
            let build_actor_id = spawn("/Users/colinrozzi/work/actors/build-actor/actor.toml", None)
                .expect("Failed to spawn build actor");
            log(&format!("Build actor ID: {}", build_actor_id));
            
            // Set up build actor channel
            let build_channel = app_state.open_build_channel(&build_actor_id, &operation_id)?;
            
            // Send initial event to client
            let initial_event = json!({
                "event_type": "operation.started",
                "source": "manager",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "sequence": 1,
                "operation_id": operation_id,
                "content": {
                    "operation_type": "build",
                    "status": "starting"
                }
            });
            
            let initial_event_bytes = serde_json::to_vec(&initial_event).map_err(|e| e.to_string())?;
            ntwk_theater_message_server_host_send_on_channel(&client_channel, &initial_event_bytes)?;
            
            // Parse runtime info
            let runtime_info_value: Value = serde_json::from_slice::<Value>(&runtime_info_response)
                .expect("Failed to parse runtime info");
                
            let runtime_info = runtime_info_value.get("data").unwrap();
            
            let cur_info = serde_json::from_value::<InfoResult>(runtime_info.clone())
                .expect("Failed to parse runtime info");
            
            // Send build request via channel
            let build_request = json!({
                "event_type": "build.request",
                "source": "manager",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "sequence": 0,
                "operation_id": operation_id,
                "content": {
                    "fs_hash": cur_info.head_hash,
                    "store_id": cur_info.store_id,
                    "build_store_id": app_state.build_store_id
                }
            });
            
            let build_request_bytes = serde_json::to_vec(&build_request).map_err(|e| e.to_string())?;
            ntwk_theater_message_server_host_send_on_channel(&build_channel, &build_request_bytes)?;
            
            // Return immediate response
            (json!({
                "status": "started", 
                "operation_id": operation_id,
                "channel_id": client_channel
            }).to_string().into_bytes(), true)
        },
        
        // Similar modifications for Start and Stop actions
        // ...
        
        // Handle channel opening requests from clients
        Action::OpenChannel { client_id, operation_id } => {
            log(&format!("Received channel open request from client: {}", client_id));
            
            // Open a channel to the client
            let channel_id = app_state.open_client_channel(&client_id, &operation_id)?;
            
            // Return the channel ID
            (json!({
                "status": "opened", 
                "channel_id": channel_id, 
                "operation_id": operation_id
            }).to_string().into_bytes(), true)
        },
    };
    
    // Save the updated state if needed
    let updated_state = if state_updated {
        let updated_state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        Some(updated_state_bytes)
    } else {
        state
    };

    Ok((updated_state, (response,)))
}
```

### 5. Channel Message Handler

Implement handlers for channel messages and closures:

```rust
// Add to the MessageServerClient implementation
fn handle_channel_open(
    state_bytes: Option<Vec<u8>>,
    params: Vec<u8>,
) -> Result<(Option<Vec<u8>>, bool, Option<Vec<u8>>), String> {
    log("Channel open request received");
    
    // Parse state
    let mut app_state: AppState = match state_bytes {
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Failed to parse state: {}", e))?,
        None => return Ok((None, false, None)),
    };
    
    // For this example, accept all incoming channels
    // In a production environment, you'd want more validation
    
    let response_message = json!({
        "event_type": "channel.accepted",
        "source": "manager",
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "content": {
            "message": "Channel accepted"
        }
    });
    
    let response_bytes = serde_json::to_vec(&response_message).map_err(|e| e.to_string())?;
    let updated_state = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
    
    Ok((Some(updated_state), true, Some(response_bytes)))
}

fn handle_channel_message(
    state_bytes: Option<Vec<u8>>,
    channel_id: String,
    msg: Vec<u8>,
) -> Result<Option<Vec<u8>>, String> {
    log(&format!("Received message on channel {}", channel_id));
    
    // Parse state
    let mut app_state: AppState = serde_json::from_slice(&state_bytes.unwrap_or_default())
        .map_err(|e| format!("Failed to parse state: {}", e))?;
    
    // Parse the message
    let event: serde_json::Value = serde_json::from_slice(&msg)
        .map_err(|e| format!("Failed to parse channel message: {}", e))?;
    
    // Route the event to other channels for the same operation
    app_state.route_event(&channel_id, &event)?;
    
    // Check for operation completion events
    if let Some(event_type) = event.get("event_type").and_then(|v| v.as_str()) {
        if event_type == "operation.completed" || event_type == "operation.failed" {
            // Get operation ID
            if let Some(operation_id) = event.get("operation_id").and_then(|v| v.as_str()) {
                // Close all channels for this operation after a delay
                // In a real implementation, you'd want to schedule this 
                // or use a timeout mechanism
                
                // For now, let's just log that we should close channels
                log(&format!("Should close channels for operation: {}", operation_id));
            }
        }
    }
    
    let updated_state = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
    Ok(Some(updated_state))
}

fn handle_channel_close(
    state_bytes: Option<Vec<u8>>,
    channel_id: String,
) -> Result<Option<Vec<u8>>, String> {
    log(&format!("Channel {} closed", channel_id));
    
    // Parse state
    let mut app_state: AppState = serde_json::from_slice(&state_bytes.unwrap_or_default())
        .map_err(|e| format!("Failed to parse state: {}", e))?;
    
    // Remove the channel from active channels
    if let Some(channel_info) = app_state.active_channels.remove(&channel_id) {
        // Remove from operation channels
        if let Some(channels) = app_state.operation_channels.get_mut(&channel_info.operation_id) {
            channels.retain(|id| id != &channel_id);
            
            // If no more channels for this operation, remove the operation
            if channels.is_empty() {
                app_state.operation_channels.remove(&channel_info.operation_id);
            }
        }
        
        // Remove from client channels if applicable
        if let Some(client_id) = channel_info.client_id {
            app_state.client_channels.remove(&client_id);
        }
    }
    
    let updated_state = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
    Ok(Some(updated_state))
}
```

### 6. Helper Functions

Add helper functions for channel management:

```rust
// Function to close all channels for an operation
fn close_operation_channels(&mut self, operation_id: &str) -> Result<(), String> {
    if let Some(channel_ids) = self.operation_channels.remove(operation_id) {
        for channel_id in channel_ids {
            if let Err(e) = ntwk_theater_message_server_host_close_channel(&channel_id) {
                log(&format!("Failed to close channel {}: {}", channel_id, e));
                // Continue trying to close other channels
            }
            
            self.active_channels.remove(&channel_id);
        }
    }
    
    Ok(())
}

// Function to send an event to all channels for an operation
fn broadcast_to_operation(&mut self, operation_id: &str, event: &serde_json::Value) -> Result<(), String> {
    if let Some(channel_ids) = self.operation_channels.get(operation_id) {
        let event_bytes = serde_json::to_vec(event).map_err(|e| e.to_string())?;
        
        for channel_id in channel_ids {
            if let Err(e) = ntwk_theater_message_server_host_send_on_channel(channel_id, &event_bytes) {
                log(&format!("Failed to send event to channel {}: {}", channel_id, e));
                // Continue trying to send to other channels
            }
        }
    }
    
    Ok(())
}
```

## Testing Requirements

1. Test channel establishment between manager and all actor types
2. Test event routing from one actor to another through the manager
3. Test handling of client disconnections
4. Test operation completion workflow with channel cleanup
5. Test concurrent operations with multiple active channels

## Acceptance Criteria

1. The manager actor successfully establishes channels with clients, the programmer actor, build actor, and child actors
2. Events are correctly routed between all participants in an operation
3. Channel resources are properly cleaned up when operations complete or channels are closed
4. The manager provides proper error handling for channel failures
5. The implementation maintains backward compatibility with non-streaming clients
