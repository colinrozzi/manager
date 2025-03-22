mod bindings;
mod messaging;
mod operations;
mod state;

use crate::bindings::exports::ntwk::theater::actor::Guest;
use crate::bindings::exports::ntwk::theater::message_server_client::{
    ChannelAccept, Guest as MessageServerClient,
};
use crate::bindings::ntwk::theater::message_server_host::{
    close_channel, request, send_on_channel,
};
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::store;
use crate::bindings::ntwk::theater::supervisor::{spawn, stop_child};
use crate::bindings::ntwk::theater::types::State;

use messaging::frontend::{FrontendCommand, FrontendMessage};
use messaging::build::{BuildActorMessage, parse_build_message};
use operations::{
    generate_operation_id, get_current_time, handle_build_command, handle_change_command,
    send_status_update,
};
use state::{AppState, InitData, OperationStatus};

use serde_json::{json, Value};

struct Actor;
impl Guest for Actor {
    fn init(state: State, params: (String,)) -> Result<(State,), String> {
        log("Initializing manager actor");
        let (param,) = params;
        log(&format!("Init parameter: {}", param));
        log(&format!("State: {:?}", state));

        let init_state =
            serde_json::from_slice::<InitData>(&state.unwrap()).map_err(|e| e.to_string())?;

        let build_store_id = match init_state.build_store_id {
            Some(id) => id,
            None => store::new().map_err(|e| e.to_string())?,
        };

        log(&format!("Build store ID: {}", build_store_id));

        let programmer_init = json!({
            "content_fs_actor_id": init_state.runtime_content_fs_actor_id,
            "anthropic_api_key": init_state.anthropic_api_key,
        });

        let programmer_actor_id = spawn(
            "/Users/colinrozzi/work/actors/programmer/manifest.toml",
            Some(&serde_json::to_vec(&programmer_init).unwrap()),
        )
        .expect("Failed to spawn programmer actor");

        log(&format!("Programmer actor ID: {}", programmer_actor_id));

        let app_state = AppState::new(
            init_state.runtime_content_fs_actor_id,
            build_store_id,
            programmer_actor_id,
        );

        let state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;

        // Create the initial state
        let new_state = Some(state_bytes);

        Ok((new_state,))
    }
}

impl MessageServerClient for Actor {
    fn handle_send(
        state: Option<Vec<u8>>,
        _params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("Handling send message");
        Ok((state,))
    }

    fn handle_request(
        state_bytes: Option<Vec<u8>>,
        _params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>, (Vec<u8>,)), String> {
        log("Handling request message");
        Ok((state_bytes, (vec![],)))
    }

    fn handle_channel_open(
        state: Option<Vec<u8>>,
        params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>, (ChannelAccept,)), String> {
        log("manager: Channel connection request received");
        let (data,) = params;

        // Parse the current state
        let state_bytes = state.unwrap_or_default();
        let _app_state: AppState = if !state_bytes.is_empty() {
            serde_json::from_slice(&state_bytes).map_err(|e| e.to_string())?
        } else {
            Err("No state found".to_string())?
        };

        // Parse the channel open message
        let open_message = match serde_json::from_slice::<Value>(&data) {
            Ok(msg) => msg,
            Err(e) => {
                log(&format!("Failed to parse channel open message: {}", e));
                return Ok((
                    Some(state_bytes),
                    (ChannelAccept {
                        accepted: false,
                        message: Some(format!("Invalid message format: {}", e).into_bytes()),
                    },),
                ));
            }
        };

        // Check if this is a frontend channel
        if let Some(client_type) = open_message.get("client_type").and_then(|v| v.as_str()) {
            if client_type == "frontend" {
                // Accept the frontend channel connection
                log("Accepting frontend channel connection");

                // Store the channel ID (will be available in handle_channel_message)
                // We'll set it when we receive the first message

                // Accept the channel
                return Ok((
                    Some(state_bytes),
                    (ChannelAccept {
                        accepted: true,
                        message: Some("Connected to manager actor. Using channel-based communication for all operations.".as_bytes().to_vec()),
                    },),
                ));
            }
        }

        // Reject other channel types
        log("Rejecting unknown channel type");
        Ok((
            Some(state_bytes),
            (ChannelAccept {
                accepted: false,
                message: Some("Unknown channel type".as_bytes().to_vec()),
            },),
        ))
    }

    fn handle_channel_message(
        state: Option<Vec<u8>>,
        params: (String, Vec<u8>),
    ) -> Result<(Option<Vec<u8>>,), String> {
        let (channel_id, message_data) = params;
        log(&format!(
            "manager: Received channel message on {}",
            channel_id
        ));

        // Parse the current state
        let state_bytes = state.unwrap_or_default();
        let mut app_state: AppState = if !state_bytes.is_empty() {
            serde_json::from_slice(&state_bytes).map_err(|e| e.to_string())?
        } else {
            Err("No state found".to_string())?
        };

        // Check if this is a message from the frontend
        if app_state.frontend_channel_id.is_none()
            || app_state.frontend_channel_id.as_ref() == Some(&channel_id)
        {
            // If frontend channel isn't set yet, set it now
            if app_state.frontend_channel_id.is_none() {
                log(&format!("Setting frontend channel ID: {}", channel_id));
                app_state.frontend_channel_id = Some(channel_id.clone());

                // Send welcome message
                let welcome_msg = FrontendMessage::Log {
                    level: "info".to_string(),
                    message: "Connected to manager actor. Using channel-based communication for all operations.".to_string(),
                };

                if let Ok(msg_bytes) = serde_json::to_vec(&welcome_msg) {
                    let _ = send_on_channel(&channel_id, &msg_bytes);
                }

                // Send initial status update
                send_status_update(&app_state, &channel_id)?;
            }

            // Try to parse the frontend command
            match serde_json::from_slice::<FrontendCommand>(&message_data) {
                Ok(command) => {
                    // Process the command
                    match process_frontend_command(&mut app_state, command, &channel_id) {
                        Ok(_) => {
                            // Command processed successfully
                            let updated_state_bytes =
                                serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
                            return Ok((Some(updated_state_bytes),));
                        }
                        Err(e) => {
                            // Send error back to frontend
                            log(&format!("Error processing command: {}", e));
                            let error_msg = FrontendMessage::Error {
                                code: "command_error".to_string(),
                                message: e,
                            };

                            if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                                let _ = send_on_channel(&channel_id, &msg_bytes);
                            }

                            let updated_state_bytes =
                                serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
                            return Ok((Some(updated_state_bytes),));
                        }
                    }
                }
                Err(e) => {
                    // Invalid command format
                    log(&format!("Invalid frontend command format: {}", e));
                    let error_msg = FrontendMessage::Error {
                        code: "invalid_command".to_string(),
                        message: format!("Invalid command format: {}", e),
                    };

                    if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                        let _ = send_on_channel(&channel_id, &msg_bytes);
                    }
                }
            }
        } else {
            // This is a message from another actor (build or programmer)
            // Find which operation this channel belongs to
            let operation_id = app_state
                .actor_channels
                .iter()
                .find(|(_, channel)| channel == &&channel_id)
                .map(|(id, _)| id.clone());

            if let Some(operation_id) = operation_id {
                log(&format!("Message from actor channel: {}", operation_id));

                // Forward the message to the frontend
                if let Some(frontend_channel) = &app_state.frontend_channel_id {
                    // Forward the message to the frontend based on operation type
                    if let Some(operation) = app_state.active_operations.get_mut(&operation_id) {
                        match operation.operation_type {
                            state::OperationType::Build => {
                                // Try to parse as a build actor message
                                match parse_build_message(&message_data) {
                                    Ok(build_msg) => {
                                        log(&format!("Received build message: {:?}", build_msg));
                                        
                                        // Extract message details
                                        let event_type = String::from(&build_msg);
                                        let message = build_msg.get_message();
                                        
                                        // Create details based on message type
                                        let details = match &build_msg {
                                            BuildActorMessage::Log { level, .. } => {
                                                json!({
                                                    "level": level
                                                })
                                            },
                                            BuildActorMessage::Progress { status, percent_complete, .. } => {
                                                json!({
                                                    "status": status,
                                                    "percent_complete": percent_complete
                                                })
                                            },
                                            BuildActorMessage::CommandStarted { command, args } => {
                                                json!({
                                                    "command": command,
                                                    "args": args
                                                })
                                            },
                                            BuildActorMessage::CommandOutput { stdout, stderr } => {
                                                json!({
                                                    "stdout": stdout,
                                                    "stderr": stderr
                                                })
                                            },
                                            BuildActorMessage::BuildComplete { success, wasm_path, wasm_hash, error } => {
                                                json!({
                                                    "success": success,
                                                    "wasm_path": wasm_path,
                                                    "wasm_hash": wasm_hash,
                                                    "error": error
                                                })
                                            }
                                        };
                                        
                                        // Create frontend message
                                        let frontend_msg = FrontendMessage::BuildEvent {
                                            operation_id: operation.operation_id.clone(),
                                            event_type,
                                            message,
                                            details,
                                        };
                                        
                                        // Send to frontend
                                        if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
                                            let _ = send_on_channel(frontend_channel, &msg_bytes);
                                        }
                                        
                                        // Check for completion event
                                        if let BuildActorMessage::BuildComplete { success, .. } = build_msg {
                                            // Update operation status
                                            operation.status = if success {
                                                OperationStatus::Completed
                                            } else {
                                                OperationStatus::Failed
                                            };
                                            operation.end_time = Some(get_current_time());
                                            
                                            // Send completion message
                                            let completion_msg = FrontendMessage::OperationCompleted {
                                                operation_id: operation.operation_id.clone(),
                                                success,
                                                message: if success {
                                                    "Build completed successfully".to_string()
                                                } else {
                                                    "Build failed".to_string()
                                                },
                                            };
                                            
                                            if let Ok(msg_bytes) = serde_json::to_vec(&completion_msg) {
                                                let _ = send_on_channel(frontend_channel, &msg_bytes);
                                            }
                                            
                                            // Close the actor channel
                                            let _ = close_channel(&channel_id);
                                            
                                            // Remove from active channels
                                            app_state.actor_channels.remove(&operation_id);
                                        }
                                    },
                                    Err(e) => {
                                        // Failed to parse build message, try generic parsing
                                        log(&format!("Failed to parse build message: {}, falling back to generic parsing", e));
                                        
                                        // Try to parse generically
                                        if let Ok(value) = serde_json::from_slice::<Value>(&message_data) {
                                            log(&format!("Received raw json: {}", value));
                                            
                                            // Create a generic message for unknown format
                                            let frontend_msg = FrontendMessage::BuildEvent {
                                                operation_id: operation.operation_id.clone(),
                                                event_type: "unknown".to_string(),
                                                message: "Build event".to_string(),
                                                details: value,
                                            };
                                            
                                            // Send to frontend
                                            if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
                                                let _ = send_on_channel(frontend_channel, &msg_bytes);
                                            }
                                        }
                                    }
                                }
                            },
                            state::OperationType::Change => {
                                // Handle programmer messages (for now use generic parsing)
                                if let Ok(value) = serde_json::from_slice::<Value>(&message_data) {
                                    let event_type = value.get("event_type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string();
                                        
                                    let content = value.get("content").cloned().unwrap_or(json!({}));
                                    let message = content
                                        .get("message")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    
                                    // Create frontend message
                                    let frontend_msg = FrontendMessage::ProgrammerEvent {
                                        operation_id: operation.operation_id.clone(),
                                        event_type,
                                        message,
                                        details: content,
                                    };
                                    
                                    // Send to frontend
                                    if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
                                        let _ = send_on_channel(frontend_channel, &msg_bytes);
                                    }
                                    
                                    // Check for completion
                                    if event_type == "TaskComplete" {
                                        let success = content
                                            .get("success")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false);
                                            
                                        // Update operation status
                                        operation.status = if success {
                                            OperationStatus::Completed
                                        } else {
                                            OperationStatus::Failed
                                        };
                                        operation.end_time = Some(get_current_time());
                                        
                                        // Send completion message
                                        let completion_msg = FrontendMessage::OperationCompleted {
                                            operation_id: operation.operation_id.clone(),
                                            success,
                                            message: if success {
                                                "Code change completed successfully".to_string()
                                            } else {
                                                "Code change failed".to_string()
                                            },
                                        };
                                        
                                        if let Ok(msg_bytes) = serde_json::to_vec(&completion_msg) {
                                            let _ = send_on_channel(frontend_channel, &msg_bytes);
                                        }
                                        
                                        // Close the actor channel
                                        let _ = close_channel(&channel_id);
                                        
                                        // Remove from active channels
                                        app_state.actor_channels.remove(&operation_id);
                                    }
                                }
                            },
                            _ => {
                                // Handle other operation types with generic message
                                if let Ok(value) = serde_json::from_slice::<Value>(&message_data) {
                                    log(&format!("Received message for operation type {:?}: {}", operation.operation_type, value));
                                    
                                    // Create generic log message
                                    let frontend_msg = FrontendMessage::Log {
                                        level: "info".to_string(),
                                        message: format!("Event from operation {}: {:?}", operation_id, value),
                                    };
                                    
                                    // Send to frontend
                                    if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
                                        let _ = send_on_channel(frontend_channel, &msg_bytes);
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                log(&format!("Unknown channel message: {}", channel_id));
            }
        }

        // Save updated state
        let updated_state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        Ok((Some(updated_state_bytes),))
    }

    fn handle_channel_close(
        state: Option<Vec<u8>>,
        params: (String,),
    ) -> Result<(Option<Vec<u8>>,), String> {
        let (channel_id,) = params;
        log(&format!("manager: Channel {} closed", channel_id));

        // Parse the current state
        let state_bytes = state.unwrap_or_default();
        let mut app_state: AppState = if !state_bytes.is_empty() {
            serde_json::from_slice(&state_bytes).map_err(|e| e.to_string())?
        } else {
            Err("No state found".to_string())?
        };

        // Check if this is the frontend channel
        if app_state
            .frontend_channel_id
            .as_ref()
            .map_or(false, |id| id == &channel_id)
        {
            log("Frontend channel closed");
            app_state.frontend_channel_id = None;

            // Close any open actor channels
            for (_, actor_channel) in &app_state.actor_channels {
                let _ = close_channel(actor_channel);
            }
            app_state.actor_channels.clear();
        } else {
            // Find and remove the actor channel
            let actor_id = app_state
                .actor_channels
                .iter()
                .find(|(_, channel)| channel == &&channel_id)
                .map(|(id, _)| id.clone());

            if let Some(id) = actor_id {
                log(&format!("Actor channel closed: {}", id));
                app_state.actor_channels.remove(&id);

                // Update operation status if it was still in progress
                if let Some(op) = app_state.active_operations.get_mut(&id) {
                    if op.status == OperationStatus::InProgress {
                        op.status = OperationStatus::Failed;
                        op.end_time = Some(get_current_time());
                    }
                }
            }
        }

        // Save updated state
        let updated_state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        Ok((Some(updated_state_bytes),))
    }
}

// Helper function to process frontend commands
fn process_frontend_command(
    app_state: &mut AppState,
    command: FrontendCommand,
    channel_id: &String,
) -> Result<(), String> {
    match command {
        FrontendCommand::StartActor => {
            log("Processing StartActor command");

            // Create operation ID and add to active operations
            let operation_id = generate_operation_id();

            // Create child manifest
            let child_manifest = r#"
name = "child"
version = "0.1.0"
description = "An HTTP server Theater actor"
component_path = "store://44768743-9232-43de-9819-47c210588b2b/wasm"

[interface]
implements = "ntwk:theater/actor"
requires = []

[[handlers]]
type = "runtime"
config = {}

[[handlers]]
type = "http-framework"
config = {}
            "#;

            // Send operation started message
            let start_msg = FrontendMessage::OperationStarted {
                operation_id: operation_id.clone(),
                operation_type: state::OperationType::Start,
                description: "Starting actor".to_string(),
            };

            if let Ok(msg_bytes) = serde_json::to_vec(&start_msg) {
                let _ = send_on_channel(channel_id, &msg_bytes);
            }

            // Spawn the child actor
            match spawn(child_manifest, None) {
                Ok(child_id) => {
                    log(&format!("Spawned child actor with ID: {}", child_id));
                    app_state.child_id = Some(child_id.clone());

                    // Record operation
                    app_state.active_operations.insert(
                        operation_id.clone(),
                        state::OperationState {
                            operation_id: operation_id.clone(),
                            operation_type: state::OperationType::Start,
                            actor_id: child_id,
                            channel_id: None, // No channel for this operation
                            status: state::OperationStatus::Completed,
                            start_time: get_current_time(),
                            end_time: Some(get_current_time()),
                        },
                    );

                    // Send completion message
                    let complete_msg = FrontendMessage::OperationCompleted {
                        operation_id,
                        success: true,
                        message: "Actor started successfully".to_string(),
                    };

                    if let Ok(msg_bytes) = serde_json::to_vec(&complete_msg) {
                        let _ = send_on_channel(channel_id, &msg_bytes);
                    }

                    Ok(())
                }
                Err(e) => {
                    // Send error message
                    let complete_msg = FrontendMessage::OperationCompleted {
                        operation_id,
                        success: false,
                        message: format!("Failed to start actor: {}", e),
                    };

                    if let Ok(msg_bytes) = serde_json::to_vec(&complete_msg) {
                        let _ = send_on_channel(channel_id, &msg_bytes);
                    }

                    Err(format!("Failed to start actor: {}", e))
                }
            }
        }
        FrontendCommand::StopActor => {
            log("Processing StopActor command");

            if let Some(child_id) = &app_state.child_id {
                // Create operation
                let operation_id = generate_operation_id();

                // Send operation started message
                let start_msg = FrontendMessage::OperationStarted {
                    operation_id: operation_id.clone(),
                    operation_type: state::OperationType::Stop,
                    description: "Stopping actor".to_string(),
                };

                if let Ok(msg_bytes) = serde_json::to_vec(&start_msg) {
                    let _ = send_on_channel(channel_id, &msg_bytes);
                }

                // Stop the child actor
                match stop_child(child_id) {
                    Ok(_) => {
                        log(&format!("Stopped child actor with ID: {}", child_id));

                        // Record operation
                        app_state.active_operations.insert(
                            operation_id.clone(),
                            state::OperationState {
                                operation_id: operation_id.clone(),
                                operation_type: state::OperationType::Stop,
                                actor_id: child_id.clone(),
                                channel_id: None,
                                status: state::OperationStatus::Completed,
                                start_time: get_current_time(),
                                end_time: Some(get_current_time()),
                            },
                        );

                        app_state.child_id = None;

                        // Send completion message
                        let complete_msg = FrontendMessage::OperationCompleted {
                            operation_id,
                            success: true,
                            message: "Actor stopped successfully".to_string(),
                        };

                        if let Ok(msg_bytes) = serde_json::to_vec(&complete_msg) {
                            let _ = send_on_channel(channel_id, &msg_bytes);
                        }

                        Ok(())
                    }
                    Err(e) => {
                        // Send error message
                        let complete_msg = FrontendMessage::OperationCompleted {
                            operation_id,
                            success: false,
                            message: format!("Failed to stop actor: {}", e),
                        };

                        if let Ok(msg_bytes) = serde_json::to_vec(&complete_msg) {
                            let _ = send_on_channel(channel_id, &msg_bytes);
                        }

                        Err(format!("Failed to stop actor: {}", e))
                    }
                }
            } else {
                // No child actor to stop
                let operation_id = generate_operation_id();

                // Send error message
                let error_msg = FrontendMessage::OperationCompleted {
                    operation_id,
                    success: false,
                    message: "No actor is currently running".to_string(),
                };

                if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                    let _ = send_on_channel(channel_id, &msg_bytes);
                }

                Err("No actor is currently running".to_string())
            }
        }
        FrontendCommand::BuildActor => {
            // Use the dedicated build command handler
            handle_build_command(app_state, channel_id)
        }
        FrontendCommand::ChangeRequest { description } => {
            // Use the dedicated change command handler
            handle_change_command(app_state, description, channel_id)
        }
        FrontendCommand::GetStatus => {
            // Send current status
            send_status_update(app_state, channel_id)?;
            Ok(())
        }
        FrontendCommand::Disconnect => {
            // Client is requesting a clean disconnect
            log("Frontend requested disconnect");

            if let Some(frontend_channel) = &app_state.frontend_channel_id {
                if frontend_channel == channel_id {
                    app_state.frontend_channel_id = None;

                    // Close any open actor channels
                    for (_, actor_channel) in &app_state.actor_channels {
                        let _ = close_channel(actor_channel);
                    }
                    app_state.actor_channels.clear();
                }
            }

            Ok(())
        }
    }
}

bindings::export!(Actor with_types_in bindings);
