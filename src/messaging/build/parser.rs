use serde_json::{Value, json};
use super::BuildActorMessage;

// Function to try to identify message type from raw JSON
pub fn parse_build_message(data: &[u8]) -> Result<BuildActorMessage, String> {
    // Try to parse as raw JSON first
    match serde_json::from_slice::<Value>(data) {
        Ok(json_value) => {
            // Try to identify message type based on fields
            if json_value.get("level").is_some() && json_value.get("message").is_some() {
                // This is a Log message
                return Ok(BuildActorMessage::Log {
                    level: json_value.get("level")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                    message: json_value.get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            } 
            else if json_value.get("status").is_some() && json_value.get("description").is_some() {
                // This is a Progress message
                return Ok(BuildActorMessage::Progress {
                    status: json_value.get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                    description: json_value.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    percent_complete: json_value.get("percent_complete")
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32),
                });
            }
            else if json_value.get("command").is_some() && json_value.get("args").is_some() {
                // This is a CommandStarted message
                let args = json_value.get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default();
                
                return Ok(BuildActorMessage::CommandStarted {
                    command: json_value.get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    args,
                });
            }
            else if json_value.get("stdout").is_some() {
                // This is a CommandOutput message
                return Ok(BuildActorMessage::CommandOutput {
                    stdout: json_value.get("stdout")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    stderr: json_value.get("stderr")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
            else if json_value.get("success").is_some() {
                // This is a BuildComplete message
                return Ok(BuildActorMessage::BuildComplete {
                    success: json_value.get("success")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    wasm_path: json_value.get("wasm_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    wasm_hash: json_value.get("wasm_hash")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    error: json_value.get("error")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
            
            // Could not determine message type
            Err(format!("Unknown message type in JSON: {}", json_value))
        },
        Err(e) => {
            Err(format!("Failed to parse JSON: {}", e))
        }
    }
}

// Extract "event" field from a wrapper if present
pub fn extract_event_from_wrapper(data: &[u8]) -> Result<Vec<u8>, String> {
    match serde_json::from_slice::<Value>(data) {
        Ok(json_value) => {
            // Check if this is an event wrapper with format {"event": {...actual_message...}}
            if let Some(event) = json_value.get("event") {
                // Return the event field as raw JSON
                match serde_json::to_vec(event) {
                    Ok(event_data) => Ok(event_data),
                    Err(e) => Err(format!("Failed to serialize event field: {}", e)),
                }
            } else {
                // Not a wrapper, return the original data
                Ok(data.to_vec())
            }
        },
        Err(e) => {
            // Not valid JSON, return the error
            Err(format!("Failed to parse JSON for event extraction: {}", e))
        }
    }
}
