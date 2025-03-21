use crate::messaging::frontend::FrontendMessage;
use crate::state::{AppState, OperationSummary};
use crate::bindings::ntwk::theater::message_server_host::send_on_channel;
use crate::bindings::ntwk::theater::runtime::log;

/// Generate a simple deterministic operation ID based on a counter
pub fn generate_operation_id() -> String {
    // Simple counter-based ID - in a real implementation this would
    // need to be stored in state, but for now this creates unique IDs
    // within a single session
    static mut COUNTER: u64 = 0;
    let id = unsafe {
        COUNTER += 1;
        COUNTER
    };
    
    format!("op-{}", id)
}

/// Get the current time (simplified for wasmtime environment)
pub fn get_current_time() -> u64 {
    // In a real implementation, we'd get the time from the system
    // For now, we'll just use a simplified counter
    static mut TIME_COUNTER: u64 = 1000; // Start at a non-zero value
    unsafe {
        TIME_COUNTER += 1;
        TIME_COUNTER
    }
}

/// Send a status update to the frontend
pub fn send_status_update(app_state: &AppState, channel_id: &String) -> Result<(), String> {
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
pub fn send_log_message(channel_id: &String, level: &str, message: &str) -> Result<(), String> {
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
pub fn send_error_message(channel_id: &String, code: &str, message: &str) -> Result<(), String> {
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
