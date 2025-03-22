use crate::bindings::ntwk::theater::message_server_host::{close_channel, send_on_channel};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messaging::build::{parse_build_message, BuildActorMessage};
use crate::messaging::frontend::{FrontendCommand, FrontendMessage};
use crate::operations::{handle_frontend_command, send_status_update};
use crate::state::{AppState, OperationStatus};
use serde_json::{json, Value};

/// Handle frontend channel setup
pub fn handle_frontend_setup(app_state: &mut AppState, channel_id: &str) -> Result<(), String> {
    log(&format!("Setting frontend channel ID: {}", channel_id));
    app_state.frontend_channel_id = Some(channel_id.to_string());
    app_state.register_channel(
        channel_id.to_string(),
        crate::messaging::ChannelType::Frontend,
    );

    // Send welcome message
    let welcome_msg = FrontendMessage::Log {
        level: "info".to_string(),
        message:
            "Connected to manager actor. Using channel-based communication for all operations."
                .to_string(),
    };

    if let Ok(msg_bytes) = serde_json::to_vec(&welcome_msg) {
        let _ = send_on_channel(&channel_id.to_string(), &msg_bytes);
    }

    // Send initial status update
    send_status_update(app_state, &channel_id.to_string())
}

/// Handle frontend message
pub fn handle_frontend_message(
    app_state: &mut AppState,
    channel_id: &str,
    message_data: &[u8],
) -> Result<(), String> {
    // Process frontend commands
    match serde_json::from_slice::<FrontendCommand>(message_data) {
        Ok(command) => {
            // Process the command
            if let Err(e) = handle_frontend_command(app_state, command, &channel_id.to_string()) {
                // Send error back to frontend
                log(&format!("Error processing command: {}", e));
                let error_msg = FrontendMessage::Error {
                    code: "command_error".to_string(),
                    message: e,
                };

                if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                    let _ = send_on_channel(&channel_id.to_string(), &msg_bytes);
                }
            }
            Ok(())
        }
        Err(e) => {
            // Invalid command format
            log(&format!("Invalid frontend command format: {}", e));
            let error_msg = FrontendMessage::Error {
                code: "invalid_command".to_string(),
                message: format!("Invalid command format: {}", e),
            };

            if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                let _ = send_on_channel(&channel_id.to_string(), &msg_bytes);
            }

            // We return Ok here to avoid terminating the channel on parse errors
            Ok(())
        }
    }
}

/// Handle build message
pub fn handle_build_message(
    app_state: &mut AppState,
    channel_id: &str,
    operation_id: &str,
    message_data: &[u8],
) -> Result<(), String> {
    // Get frontend channel
    let frontend_channel = match &app_state.frontend_channel_id {
        Some(id) => id,
        None => {
            log("Warning: Received build message but no frontend channel is available");
            // We can still process the message, just can't forward to frontend
            return Ok(());
        }
    };

    // Get operation state
    let operation = match app_state.active_operations.get_mut(operation_id) {
        Some(op) => op,
        None => {
            log(&format!(
                "Operation {} not found for build message",
                operation_id
            ));
            return Ok(());
        }
    };

    // Try to parse as build message
    match parse_build_message(message_data) {
        Ok(build_msg) => {
            log(&format!("Received build message: {:?}", build_msg));

            // Extract message details
            let event_type = String::from(&build_msg);
            let message = build_msg.get_message();

            // Create details based on message type
            let details = match build_msg {
                BuildActorMessage::Log { level, .. } => {
                    json!({
                        "level": level
                    })
                }
                BuildActorMessage::Progress {
                    status,
                    percent_complete,
                    ..
                } => {
                    json!({
                        "status": status,
                        "percent_complete": percent_complete
                    })
                }
                BuildActorMessage::CommandStarted { command, args } => {
                    json!({
                        "command": command,
                        "args": args
                    })
                }
                BuildActorMessage::CommandOutput { stdout, stderr } => {
                    json!({
                        "stdout": stdout,
                        "stderr": stderr
                    })
                }
                BuildActorMessage::BuildComplete {
                    success,
                    wasm_path,
                    wasm_hash,
                    error,
                } => {
                    log(&format!(
                        "Build completed with success: {:?}, wasm_path: {:?}, wasm_hash: {:?}, error: {:?}",
                        success, wasm_path, wasm_hash, error
                    ));
                    // Update operation status
                    if success {
                        OperationStatus::Completed
                    } else {
                        OperationStatus::Failed
                    };

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
                    let _ = close_channel(&channel_id.to_string());

                    // Remove from channels
                    app_state.channels.remove(channel_id);
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

            Ok(())
        }
        Err(e) => {
            // Failed to parse build message, try generic parsing
            log(&format!(
                "Failed to parse build message: {}, falling back to generic parsing",
                e
            ));

            // Try to parse generically
            if let Ok(value) = serde_json::from_slice::<Value>(message_data) {
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

            Ok(())
        }
    }
}

/// Handle programmer message
pub fn handle_programmer_message(
    app_state: &mut AppState,
    channel_id: &str,
    operation_id: &str,
    message_data: &[u8],
) -> Result<(), String> {
    // Get frontend channel
    let frontend_channel = match &app_state.frontend_channel_id {
        Some(id) => id,
        None => {
            log("Warning: Received programmer message but no frontend channel is available");
            // We can still process the message, just can't forward to frontend
            return Ok(());
        }
    };

    // Get operation state
    let operation = match app_state.active_operations.get_mut(operation_id) {
        Some(op) => op,
        None => {
            log(&format!(
                "Operation {} not found for programmer message",
                operation_id
            ));
            return Ok(());
        }
    };

    // Handle programmer messages (using generic parsing)
    if let Ok(value) = serde_json::from_slice::<Value>(message_data) {
        let event_type = value
            .get("event_type")
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
            event_type: event_type.clone(),
            message,
            details: content.clone(),
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
            let _ = close_channel(&channel_id.to_string());

            // Remove from channels
            app_state.channels.remove(channel_id);
        }
    }

    Ok(())
}

/// Handle unknown channel message
pub fn handle_unknown_message(
    app_state: &mut AppState,
    channel_id: &str,
    message_data: &[u8],
) -> Result<(), String> {
    log(&format!(
        "Received message from unknown channel type: {}",
        channel_id
    ));

    // Try to parse as JSON for logging
    if let Some(frontend_channel) = &app_state.frontend_channel_id {
        if let Ok(value) = serde_json::from_slice::<Value>(message_data) {
            // Create generic log message
            let frontend_msg = FrontendMessage::Log {
                level: "info".to_string(),
                message: format!("Message from unknown channel {}: {:?}", channel_id, value),
            };

            // Send to frontend
            if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
                let _ = send_on_channel(frontend_channel, &msg_bytes);
            }
        }
    }

    Ok(())
}
