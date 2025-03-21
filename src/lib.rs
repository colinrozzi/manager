mod bindings;
mod state;
mod messaging;
mod operations;

use crate::bindings::exports::ntwk::theater::actor::Guest;
use crate::bindings::exports::ntwk::theater::message_server_client::{
    self, ChannelAccept, Guest as MessageServerClient,
};
use crate::bindings::ntwk::theater::message_server_host::{
    close_channel, request, send_on_channel,
};
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::store;
use crate::bindings::ntwk::theater::supervisor::{spawn, stop_child};
use crate::bindings::ntwk::theater::types::State;

use state::{Action, AppState, InitData, OperationStatus};
use messaging::frontend::{FrontendCommand, FrontendMessage};
use operations::{
    generate_operation_id, get_current_time, send_status_update,
    handle_build_command, handle_change_command
};

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
            programmer_actor_id
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
        params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>,), String> {
        log("Handling send message");
        let (data,) = params;

        // Parse the current state
        let state_bytes = state.unwrap_or_default();
        let app_state: AppState = if !state_bytes.is_empty() {
            serde_json::from_slice(&state_bytes).map_err(|e| e.to_string())?
        } else {
            Err("No state found".to_string())?
        };

        // Try to parse the message as a string
        if let Ok(message) = String::from_utf8(data.clone()) {
            log(&format!("Received message: {}", message));
        }

        // Save the updated state
        let updated_state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        let updated_state = Some(updated_state_bytes);

        Ok((updated_state,))
    }

    fn handle_request(
        state: Option<Vec<u8>>,
        params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>, (Vec<u8>,)), String> {
        log("Handling request message");
        let (data,) = params;

        // Parse the current state
        let state_bytes = state.unwrap_or_default();
        let mut app_state: AppState = if !state_bytes.is_empty() {
            serde_json::from_slice(&state_bytes).map_err(|e| e.to_string())?
        } else {
            Err("No state found".to_string())?
        };

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

        let action: Action = serde_json::from_slice(&data).map_err(|e| e.to_string())?;

        // If we have a frontend channel, we should recommend using it
        if app_state.frontend_channel_id.is_some() {
            log("Warning: Using request/response while a frontend channel is active. Channel-based operations provide better progress updates.");
        }

        let response = match action {
            Action::Start => {
                let child_id = spawn(child_manifest, None).map_err(|e| e.to_string())?;
                log(&format!("Spawned child actor with ID: {}", child_id));
                app_state.child_id = Some(child_id);
                "Started child actor".as_bytes().to_vec()
            }
            Action::Stop => {
                if let Some(child_id) = app_state.child_id.take() {
                    log(&format!("Stopping child actor with ID: {}", child_id));
                    stop_child(&child_id).map_err(|e| e.to_string())?;
                    app_state.child_id = None;
                    "Stopped child actor".as_bytes().to_vec()
                } else {
                    "No child actor to stop".as_bytes().to_vec()
                }
            }
            Action::Build => {
                let runtime_info_response = request(
                    &app_state.runtime_content_fs_actor_id,
                    &serde_json::to_vec(&json!({"action": "get-info", "params": []})).unwrap(),
                )
                .expect("Failed to get programmer actor info");

                log(&format!(
                    "Received runtime info: {}",
                    String::from_utf8(runtime_info_response.clone()).unwrap()
                ));

                let build_actor_id =
                    spawn("/Users/colinrozzi/work/actors/build-actor/actor.toml", None)
                        .expect("Failed to spawn build actor");
                log(&format!("Build actor ID: {}", build_actor_id.clone()));

                let runtime_info_value: Value =
                    serde_json::from_slice::<Value>(&runtime_info_response)
                        .expect("Failed to parse runtime info");

                log(&format!("Runtime info value: {:?}", runtime_info_value));

                let runtime_info = runtime_info_value.get("data").unwrap();

                log(&format!("Runtime info: {:?}", runtime_info));

                let cur_info = serde_json::from_value::<state::InfoResult>(runtime_info.clone())
                    .expect("Failed to parse programmer actor info");
                log(&format!("Programmer actor response: {:?}", cur_info));

                let build_state = json!({
                    "fs_hash": cur_info.head_hash,
                    "store_id": cur_info.store_id,
                    "build_store_id": app_state.build_store_id,
                });
                let result = request(&build_actor_id, &serde_json::to_vec(&build_state).unwrap())
                    .map_err(|e| e.to_string())?;
                log(&format!(
                    "Build actor response: {}",
                    String::from_utf8(result.clone()).unwrap()
                ));

                let result: state::BuildOutput =
                    serde_json::from_slice(&result).map_err(|e| e.to_string())?;
                log(&format!("Build output: {:?}", result));

                let bytes = store::get_by_label(&app_state.build_store_id, "wasm")
                    .map_err(|e| e.to_string())?;

                log(&format!("Wasm bytes: {:?}", bytes));

                stop_child(&build_actor_id).map_err(|e| e.to_string())?;

                "Built".as_bytes().to_vec()
            }
            Action::Change(req) => {
                log(&format!("Received change request: {}", req));

                let result = request(
                    &app_state.programmer_actor_id,
                    &serde_json::to_vec(&json!({"change": req})).unwrap(),
                )
                .expect("Failed to send change request");

                log(&format!(
                    "Received programmer actor response: {}",
                    String::from_utf8(result.clone()).unwrap()
                ));

                "Changed".as_bytes().to_vec()
            }
        };

        // Save the updated state
        let updated_state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
        let updated_state = Some(updated_state_bytes);

        Ok((updated_state, (response,)))
    }
    
    // Channel handlers
    
    fn handle_channel_open(
        state: Option<Vec<u8>>,
        params: (Vec<u8>,),
    ) -> Result<(Option<Vec<u8>>, (ChannelAccept,)), String> {
        log("manager: Channel connection request received");
        let (data,) = params;
        
        // Parse the current state
        let state_bytes = state.unwrap_or_default();
        let app_state: AppState = if !state_bytes.is_empty() {
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
                        message: Some("Connected to manager actor".as_bytes().to_vec()),
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
        log(&format!("manager: Received channel message on {}", channel_id));
        
        // Parse the current state
        let state_bytes = state.unwrap_or_default();
        let mut app_state: AppState = if !state_bytes.is_empty() {
            serde_json::from_slice(&state_bytes).map_err(|e| e.to_string())?
        } else {
            Err("No state found".to_string())?
        };
        
        // Check if this is a message from the frontend
        if app_state.frontend_channel_id.is_none() || app_state.frontend_channel_id.as_ref().unwrap() == &channel_id {
            // If frontend channel isn't set yet, set it now
            if app_state.frontend_channel_id.is_none() {
                log(&format!("Setting frontend channel ID: {}", channel_id));
                app_state.frontend_channel_id = Some(channel_id.clone());
                
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
                            let updated_state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
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
                            
                            let updated_state_bytes = serde_json::to_vec(&app_state).map_err(|e| e.to_string())?;
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
            let operation_id = app_state.actor_channels.iter()
                .find(|(_, channel)| channel == &&channel_id)
                .map(|(id, _)| id.clone());
            
            if let Some(operation_id) = operation_id {
                log(&format!("Message from actor channel: {}", operation_id));
                
                // Forward the message to the frontend
                if let Some(frontend_channel) = &app_state.frontend_channel_id {
                    // Parse the message and create appropriate frontend message
                    if let Ok(actor_message) = serde_json::from_slice::<Value>(&message_data) {
                        if let Some(operation) = app_state.active_operations.get(&operation_id) {
                            let event_type = actor_message.get("event_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            
                            let content = actor_message.get("content").cloned().unwrap_or(json!({}));
                            let message = content.get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            
                            // Create appropriate message type based on actor
                            let frontend_msg = match operation.operation_type {
                                state::OperationType::Build => FrontendMessage::BuildEvent {
                                    operation_id: operation.operation_id.clone(),
                                    event_type: event_type.to_string(),
                                    message,
                                    details: content,
                                },
                                state::OperationType::Change => FrontendMessage::ProgrammerEvent {
                                    operation_id: operation.operation_id.clone(),
                                    event_type: event_type.to_string(),
                                    message,
                                    details: content,
                                },
                                _ => {
                                    // Generic log message for other types
                                    FrontendMessage::Log {
                                        level: "info".to_string(),
                                        message: format!("{}: {}", event_type, message),
                                    }
                                }
                            };
                            
                            // Send to frontend
                            if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
                                let _ = send_on_channel(frontend_channel, &msg_bytes);
                            }
                            
                            // Check for completion events
                            if event_type == "BuildComplete" || event_type == "TaskComplete" {
                                let success = content.get("success")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                
                                // Update operation status
                                if let Some(op) = app_state.active_operations.get_mut(&operation_id) {
                                    op.status = if success { 
                                        OperationStatus::Completed 
                                    } else { 
                                        OperationStatus::Failed 
                                    };
                                    op.end_time = Some(get_current_time());
                                }
                                
                                // Send completion message
                                let completion_msg = FrontendMessage::OperationCompleted {
                                    operation_id: operation.operation_id.clone(),
                                    success,
                                    message: if success {
                                        format!("{} completed successfully", 
                                            if operation.operation_type == state::OperationType::Build {
                                                "Build"
                                            } else {
                                                "Code change"
                                            }
                                        )
                                    } else {
                                        format!("{} failed", 
                                            if operation.operation_type == state::OperationType::Build {
                                                "Build"
                                            } else {
                                                "Code change"
                                            }
                                        )
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
        if app_state.frontend_channel_id.as_ref().map_or(false, |id| id == &channel_id) {
            log("Frontend channel closed");
            app_state.frontend_channel_id = None;
            
            // Close any open actor channels
            for (_, actor_channel) in &app_state.actor_channels {
                let _ = close_channel(actor_channel);
            }
            app_state.actor_channels.clear();
        } else {
            // Find and remove the actor channel
            let actor_id = app_state.actor_channels.iter()
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
    channel_id: &str,
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
