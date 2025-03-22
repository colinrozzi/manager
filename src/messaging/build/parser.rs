use serde_json::{Value, json};
use super::BuildActorMessage;

// Function to try to identify message type from raw JSON
pub fn parse_build_message(data: &[u8]) -> Result<BuildActorMessage, String> {
    // Try to parse as raw JSON first
    match serde_json::from_slice::<Value>(data) {
        Ok(json_value) => {
            // First check if this is a wrapped enum message (matches build-actor format)
            // The format is expected to be something like: {"Log": {"level": "Info", "message": "..."}}            
            if let Some(log) = json_value.get("Log") {
                if let Some(level) = log.get("level") {
                    // Get the level as a string, handling both string and object formats
                    let level_str = if level.is_string() {
                        level.as_str().unwrap_or("Unknown").to_string()
                    } else {
                        // This might be an enum variant like {"Info": null}
                        let level_keys: Vec<String> = level.as_object()
                            .map(|obj| obj.keys().cloned().collect())
                            .unwrap_or_default();
                        
                        if !level_keys.is_empty() {
                            level_keys[0].clone()
                        } else {
                            "Unknown".to_string()
                        }
                    };
                    
                    return Ok(BuildActorMessage::Log {
                        level: level_str,
                        message: log.get("message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            } 
            
            if let Some(progress) = json_value.get("Progress") {
                return Ok(BuildActorMessage::Progress {
                    status: progress.get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                    description: progress.get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    percent_complete: progress.get("percent_complete")
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32),
                });
            }
            
            if let Some(cmd) = json_value.get("CommandStarted") {
                let args = cmd.get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default();
                
                return Ok(BuildActorMessage::CommandStarted {
                    command: cmd.get("command")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    args,
                });
            }
            
            if let Some(output) = json_value.get("CommandOutput") {
                return Ok(BuildActorMessage::CommandOutput {
                    stdout: output.get("stdout")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    stderr: output.get("stderr")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
            
            if let Some(build) = json_value.get("BuildComplete") {
                return Ok(BuildActorMessage::BuildComplete {
                    success: build.get("success")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    wasm_path: build.get("wasm_path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    wasm_hash: build.get("wasm_hash")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    error: build.get("error")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
            
            if let Some(file) = json_value.get("FileExtracted") {
                return Ok(BuildActorMessage::FileExtracted {
                    path: file.get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    size: file.get("size")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize,
                });
            }

            // Fallback to the old parsing method (direct field check)
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
