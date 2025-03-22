use crate::bindings::ntwk::theater::message_server_host::{open_channel, send_on_channel};
use crate::bindings::ntwk::theater::runtime::log;
use crate::messaging::frontend::FrontendMessage;
use crate::state::{AppState, OperationState, OperationStatus, OperationType};
use crate::operations::utils::{generate_operation_id, get_current_time};

use serde_json::json;

/// Handle a change command from the frontend
pub fn handle_change_command(
    app_state: &mut AppState, 
    description: String, 
    frontend_channel_id: &String
) -> Result<(), String> {
    log(&format!("Processing ChangeRequest command: {}", description));
    
    // Generate a unique operation ID
    let operation_id = generate_operation_id();
    
    // Send operation started message to frontend
    let start_msg = FrontendMessage::OperationStarted {
        operation_id: operation_id.clone(),
        operation_type: OperationType::Change,
        description: format!("Starting code change: {}", description),
    };
    
    if let Ok(msg_bytes) = serde_json::to_vec(&start_msg) {
        let _ = send_on_channel(frontend_channel_id, &msg_bytes);
    }
    
    // Initialize channel request message
    let channel_init = json!({
        "operation_id": operation_id,
        "type": "change"
    });
    
    // Open a channel to the programmer actor
    let channel_init_bytes = match serde_json::to_vec(&channel_init) {
        Ok(bytes) => bytes,
        Err(e) => {
            let error_msg = format!("Failed to serialize channel init: {}", e);
            log(&error_msg);
            
            // Send error to frontend
            let frontend_msg = FrontendMessage::OperationCompleted {
                operation_id: operation_id.clone(),
                success: false,
                message: error_msg.clone(),
            };
            
            if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }
            
            return Err(error_msg);
        }
    };
    
    // Open channel to programmer actor
    let programmer_channel_id = match open_channel(&app_state.programmer_actor_id, &channel_init_bytes) {
        Ok(id) => id,
        Err(e) => {
            let error_msg = format!("Failed to open channel to programmer actor: {}", e);
            log(&error_msg);
            
            // Send error to frontend
            let frontend_msg = FrontendMessage::OperationCompleted {
                operation_id: operation_id.clone(),
                success: false,
                message: error_msg.clone(),
            };
            
            if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }
            
            return Err(error_msg);
        }
    };
    
    log(&format!("Opened channel to programmer actor: {}", programmer_channel_id));
    
    // Register the operation
    app_state.active_operations.insert(
        operation_id.clone(),
        OperationState {
            operation_id: operation_id.clone(),
            operation_type: OperationType::Change,
            actor_id: app_state.programmer_actor_id.clone(),
            channel_id: Some(programmer_channel_id.clone()),
            status: OperationStatus::InProgress,
            start_time: get_current_time(),
            end_time: None,
        },
    );
    
    // Register the programmer channel
    app_state.register_channel(
        programmer_channel_id.clone(),
        crate::messaging::ChannelType::Programmer {
            operation_id: operation_id.clone(),
        }
    );
    
    // Send the change command
    let change_command = json!({
        "command": "start_change",
        "change": description
    });
    
    let change_command_bytes = match serde_json::to_vec(&change_command) {
        Ok(bytes) => bytes,
        Err(e) => {
            let error_msg = format!("Failed to serialize change command: {}", e);
            log(&error_msg);
            
            // Send error to frontend
            let frontend_msg = FrontendMessage::OperationCompleted {
                operation_id: operation_id.clone(),
                success: false,
                message: error_msg.clone(),
            };
            
            if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }
            
            // Clean up
            app_state.actor_channels.remove(&operation_id);
            app_state.active_operations.remove(&operation_id);
            
            return Err(error_msg);
        }
    };
    
    // Send the change command to the programmer actor
    if let Err(e) = send_on_channel(&programmer_channel_id, &change_command_bytes) {
        let error_msg = format!("Failed to send change command: {}", e);
        log(&error_msg);
        
        // Send error to frontend
        let frontend_msg = FrontendMessage::OperationCompleted {
            operation_id: operation_id.clone(),
            success: false,
            message: error_msg.clone(),
        };
        
        if let Ok(msg_bytes) = serde_json::to_vec(&frontend_msg) {
            let _ = send_on_channel(frontend_channel_id, &msg_bytes);
        }
        
        // Clean up
        app_state.actor_channels.remove(&operation_id);
        app_state.active_operations.remove(&operation_id);
        
        return Err(error_msg);
    }
    
    log(&format!("Sent change command to programmer actor"));
    
    // Send a progress update to the frontend
    let progress_msg = FrontendMessage::OperationProgress {
        operation_id: operation_id.clone(),
        description: "Code change started, waiting for updates...".to_string(),
        percent_complete: Some(5.0),
    };
    
    if let Ok(msg_bytes) = serde_json::to_vec(&progress_msg) {
        let _ = send_on_channel(frontend_channel_id, &msg_bytes);
    }
    
    // The rest will happen asynchronously via channel messages
    Ok(())
}
