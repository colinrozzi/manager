use crate::handle_build_command;
use crate::handle_change_command;
use crate::operations::utils::{generate_operation_id, send_status_update};
use crate::state;
use crate::state::AppState;
use crate::FrontendCommand;
use crate::FrontendMessage;

use crate::bindings::ntwk::theater::message_server_host::{close_channel, send_on_channel};
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::supervisor::{spawn, stop_child};

pub fn handle_frontend_command(
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
                            actor_id: child_id.clone(),
                            channel_id: None, // No channel for this operation
                            status: state::OperationStatus::Completed,
                        },
                    );

                    let child_started_msg = FrontendMessage::ChildStarted {
                        child_id: child_id.clone(),
                    };

                    log(&format!(
                        "Sending ChildStarted message: {:?}",
                        child_started_msg
                    ));

                    if let Ok(msg_bytes) = serde_json::to_vec(&child_started_msg) {
                        let _ = send_on_channel(channel_id, &msg_bytes);
                    }

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
                            },
                        );

                        // Send completion message
                        let complete_msg = FrontendMessage::OperationCompleted {
                            operation_id,
                            success: true,
                            message: "Actor stopped successfully".to_string(),
                        };

                        if let Ok(msg_bytes) = serde_json::to_vec(&complete_msg) {
                            let _ = send_on_channel(channel_id, &msg_bytes);
                        }

                        let child_stopped_msg = FrontendMessage::ChildStopped {
                            child_id: child_id.clone(),
                        };

                        if let Ok(msg_bytes) = serde_json::to_vec(&child_stopped_msg) {
                            let _ = send_on_channel(channel_id, &msg_bytes);
                        }

                        app_state.child_id = None;

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
