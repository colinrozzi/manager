use crate::bindings::ntwk::theater::message_server_host::{close_channel, send_on_channel};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messaging::frontend::FrontendMessage;
use crate::state::{AppState, OperationStatus};
use serde_json::{json, Value};

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

    // Parse the message from programmer actor
    if let Ok(message_value) = serde_json::from_slice::<serde_json::Value>(message_data) {
        log(&format!("Received programmer message: {}", message_value));

        // Determine message type from the JSON structure
        let message_type = message_value
            .as_object()
            .and_then(|obj| obj.keys().next().map(|key| key.as_str()))
            .unwrap_or("unknown");

        let event_type = message_type.to_string();

        // Extract the content based on message type
        let content = match message_type {
            "Progress" => {
                // Handle Progress message
                let stage = message_value["Progress"]["stage"]
                    .as_str()
                    .unwrap_or("Unknown");
                let description = message_value["Progress"]["description"]
                    .as_str()
                    .unwrap_or("");
                let percent = message_value["Progress"]["percent_complete"].as_f64();

                // Update operation progress in frontend
                let progress_msg = FrontendMessage::OperationProgress {
                    operation_id: operation.operation_id.clone(),
                    description: description.to_string(),
                    percent_complete: percent.map(|p| p as f32),
                };

                if let Ok(msg_bytes) = serde_json::to_vec(&progress_msg) {
                    let _ = send_on_channel(frontend_channel, &msg_bytes);
                }

                // Return content for ProgrammerEvent
                json!({
                    "stage": stage,
                    "description": description,
                    "percent_complete": percent,
                    "message": description
                })
            }
            "Log" => {
                // Handle Log message
                let level = message_value["Log"]["level"].as_str().unwrap_or("info");
                let message = message_value["Log"]["message"].as_str().unwrap_or("");

                // Create a log message for the frontend
                let log_msg = FrontendMessage::Log {
                    level: level.to_string(),
                    message: message.to_string(),
                };

                if let Ok(msg_bytes) = serde_json::to_vec(&log_msg) {
                    let _ = send_on_channel(frontend_channel, &msg_bytes);
                }

                // Return content for ProgrammerEvent
                json!({
                    "level": level,
                    "message": message
                })
            }
            "AssistantResponse" => {
                // Handle AssistantResponse message
                let content = message_value["AssistantResponse"]["content"]
                    .as_str()
                    .unwrap_or("");
                let is_final = message_value["AssistantResponse"]["is_final"]
                    .as_bool()
                    .unwrap_or(false);

                json!({
                    "content": content,
                    "is_final": is_final,
                    "message": "Assistant response received"
                })
            }
            "ToolUse" => {
                // Handle ToolUse message
                let id = message_value["ToolUse"]["id"].as_str().unwrap_or("");
                let name = message_value["ToolUse"]["name"].as_str().unwrap_or("");
                let input = &message_value["ToolUse"]["input"];

                json!({
                    "id": id,
                    "name": name,
                    "input": input,
                    "message": format!("Using tool: {}", name)
                })
            }
            "ToolResult" => {
                // Handle ToolResult message
                let tool_use_id = message_value["ToolResult"]["tool_use_id"]
                    .as_str()
                    .unwrap_or("");
                let content = message_value["ToolResult"]["content"]
                    .as_str()
                    .unwrap_or("");

                json!({
                    "tool_use_id": tool_use_id,
                    "content": content,
                    "message": format!("Tool result for ID: {}", tool_use_id)
                })
            }
            "TaskComplete" => {
                // Handle TaskComplete message
                let success = message_value["TaskComplete"]["success"]
                    .as_bool()
                    .unwrap_or(false);
                let error = message_value["TaskComplete"]["error"].as_str();

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
                    } else if let Some(err) = error {
                        format!("Code change failed: {}", err)
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

                // Return content for ProgrammerEvent
                json!({
                    "success": success,
                    "error": error,
                    "message": if success { "Task completed successfully" } else { "Task failed" }
                })
            }
            _ => {
                // Unknown message type
                json!({
                    "unknown_message": message_value,
                    "message": "Unknown message type received"
                })
            }
        };

        // Create frontend message
        let frontend_msg = FrontendMessage::ProgrammerEvent {
            operation_id: operation.operation_id.clone(),
            event_type,
            message: content
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            details: content,
        };

        // Send to frontend
        if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
            let _ = send_on_channel(frontend_channel, &msg_bytes);
        }
    } else {
        // Failed to parse as JSON
        log("Failed to parse programmer message as JSON");
    }

    Ok(())
}





                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                










                    .as_bool()
                    


                    .as_str()
                    .unwrap_or("");


                    .as_str()
                    





                    .as_bool()         )
            .as_object()
            
    
            .as_str()
                    






                    .as_str()
                    

                    .as_str()
                    

