use crate::messaging::frontend::FrontendMessage;
use crate::state::{AppState, OperationSummary};
use crate::bindings::ntwk::theater::message_server_host::send_on_channel;
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::timing::get_system_time;

use rand::{distributions::Alphanumeric, Rng};
use serde_json::json;

/// Generate a unique operation ID
pub fn generate_operation_id() -> String {
    let timestamp = get_current_time();
    let random_suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    
    format!("op-{}-{}", timestamp, random_suffix)
}

/// Get the current system time in milliseconds
pub fn get_current_time() -> u64 {
    get_system_time()
}

/// Send a status update to the frontend
pub fn send_status_update(app_state: &AppState, channel_id: &str) -> Result<(), String> {
    log("Sending status update to frontend");
    
    let active_operations: Vec<OperationSummary> = app_state.active_operations.values()
        .map(|op| op.into())
        .collect();
    
    let status_msg = FrontendMessage::Status {
        child_running: app_state.child_id.is_some(),
        active_operations,
    };
    
    match serde_json::to_vec(&status_msg) {
        Ok(msg_bytes) => {
            send_on_channel(channel_id, &msg_bytes).map_err(|e| e.to_string())?;
            Ok(())
        },
        Err(e) => Err(format!("Failed to serialize status message: {}", e)),
    }
}

/// Send a log message to the frontend
pub fn send_log_message(channel_id: &str, level: &str, message: &str) -> Result<(), String> {
    let log_msg = FrontendMessage::Log {
        level: level.to_string(),
        message: message.to_string(),
    };
    
    match serde_json::to_vec(&log_msg) {
        Ok(msg_bytes) => {
            send_on_channel(channel_id, &msg_bytes).map_err(|e| e.to_string())?;
            Ok(())
        },
        Err(e) => Err(format!("Failed to serialize log message: {}", e)),
    }
}

/// Send an error message to the frontend
pub fn send_error_message(channel_id: &str, code: &str, message: &str) -> Result<(), String> {
    let error_msg = FrontendMessage::Error {
        code: code.to_string(),
        message: message.to_string(),
    };
    
    match serde_json::to_vec(&error_msg) {
        Ok(msg_bytes) => {
            send_on_channel(channel_id, &msg_bytes).map_err(|e| e.to_string())?;
            Ok(())
        },
        Err(e) => Err(format!("Failed to serialize error message: {}", e)),
    }
}
