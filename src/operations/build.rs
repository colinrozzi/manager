use crate::bindings::ntwk::theater::message_server_host::{open_channel, request, send_on_channel};
use crate::bindings::ntwk::theater::runtime::log;
use crate::bindings::ntwk::theater::supervisor::spawn;
use crate::messaging::frontend::FrontendMessage;
use crate::operations::utils::{generate_operation_id, get_current_time};
use crate::state::{AppState, InfoResult, OperationState, OperationStatus, OperationType};

use serde_json::{json, Value};

/// Handle a build command from the frontend
pub fn handle_build_command(
    app_state: &mut AppState,
    frontend_channel_id: &String,
) -> Result<(), String> {
    log("Processing BuildActor command");

    // Generate a unique operation ID
    let operation_id = generate_operation_id();

    // Send operation started message to frontend
    let start_msg = FrontendMessage::OperationStarted {
        operation_id: operation_id.clone(),
        operation_type: OperationType::Build,
        description: "Starting build process".to_string(),
    };

    if let Ok(msg_bytes) = serde_json::to_vec(&start_msg) {
        let _ = send_on_channel(frontend_channel_id, &msg_bytes);
    }

    // Get file system information from content-fs actor
    let runtime_info_response = match request(
        &app_state.runtime_content_fs_actor_id,
        &serde_json::to_vec(&json!({"action": "get-info", "params": []})).unwrap(),
    ) {
        Ok(resp) => resp,
        Err(e) => {
            let error_msg = format!("Failed to get filesystem info: {}", e);
            log(&error_msg);

            // Send error to frontend
            let error_msg = FrontendMessage::OperationCompleted {
                operation_id,
                success: false,
                message: error_msg,
            };

            if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }

            return Err(format!("Failed to get filesystem info: {}", e));
        }
    };

    log(&format!("Received runtime info"));

    // Parse the response
    let runtime_info_value: Value = match serde_json::from_slice(&runtime_info_response) {
        Ok(val) => val,
        Err(e) => {
            let error_msg = format!("Failed to parse filesystem info: {}", e);
            log(&error_msg);

            // Send error to frontend
            let error_msg = FrontendMessage::OperationCompleted {
                operation_id,
                success: false,
                message: error_msg,
            };

            if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }

            return Err(format!("Failed to parse filesystem info: {}", e));
        }
    };

    let runtime_info = match runtime_info_value.get("data") {
        Some(data) => data,
        None => {
            let error_string = "Missing data field in filesystem info";
            log(error_string);

            // Send error to frontend
            let error_msg = FrontendMessage::OperationCompleted {
                operation_id,
                success: false,
                message: error_string.to_string(),
            };

            if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }

            return Err(error_string.to_string());
        }
    };

    // Parse filesystem info
    let fs_info: InfoResult = match serde_json::from_value(runtime_info.clone()) {
        Ok(info) => info,
        Err(e) => {
            let error_msg = format!("Failed to parse filesystem info data: {}", e);
            log(&error_msg);

            // Send error to frontend
            let error_msg = FrontendMessage::OperationCompleted {
                operation_id,
                success: false,
                message: error_msg.clone(),
            };

            if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }

            return Err(error_msg);
        }
    };

    log(&format!(
        "Filesystem info: hash={}, store_id={}",
        fs_info.head_hash, fs_info.store_id
    ));

    // Spawn the build actor
    let build_actor_id = match spawn("/Users/colinrozzi/work/actors/build-actor/actor.toml", None) {
        Ok(id) => id,
        Err(e) => {
            let error_msg = format!("Failed to spawn build actor: {}", e);
            log(&error_msg);

            // Send error to frontend
            let error_msg = FrontendMessage::OperationCompleted {
                operation_id,
                success: false,
                message: error_msg.clone(),
            };

            if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }

            return Err(error_msg);
        }
    };

    log(&format!("Spawned build actor with ID: {}", build_actor_id));

    // Register the operation
    app_state.active_operations.insert(
        operation_id.clone(),
        OperationState {
            operation_id: operation_id.clone(),
            operation_type: OperationType::Build,
            actor_id: build_actor_id.clone(),
            channel_id: None, // Will set this after opening channel
            status: OperationStatus::Pending,
            start_time: get_current_time(),
            end_time: None,
        },
    );

    // Initialize channel request message
    let channel_init = json!({
        "operation_id": operation_id,
        "type": "build"
    });

    // Open a channel to the build actor
    let channel_init_bytes = match serde_json::to_vec(&channel_init) {
        Ok(bytes) => bytes,
        Err(e) => {
            let error_msg = format!("Failed to serialize channel init: {}", e);
            log(&error_msg);

            // Send error to frontend
            let error_msg = FrontendMessage::OperationCompleted {
                operation_id: operation_id.clone(),
                success: false,
                message: error_msg.clone(),
            };

            if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }

            // Remove the operation from active operations
            app_state.active_operations.remove(&operation_id);

            return Err(error_msg);
        }
    };

    // Open channel to build actor
    let build_channel_id = match open_channel(&build_actor_id, &channel_init_bytes) {
        Ok(id) => id,
        Err(e) => {
            let error_msg = format!("Failed to open channel to build actor: {}", e);
            log(&error_msg);

            // Send error to frontend
            let error_msg = FrontendMessage::OperationCompleted {
                operation_id: operation_id.clone(),
                success: false,
                message: error_msg.clone(),
            };

            if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }

            // Remove the operation from active operations
            app_state.active_operations.remove(&operation_id);

            return Err(error_msg);
        }
    };

    log(&format!(
        "Opened channel to build actor: {}",
        build_channel_id
    ));

    // Update the operation with the channel ID
    if let Some(op) = app_state.active_operations.get_mut(&operation_id) {
        op.channel_id = Some(build_channel_id.clone());
        op.status = OperationStatus::InProgress;
    }

    // Store the mapping from operation ID to channel ID
    app_state
        .actor_channels
        .insert(operation_id.clone(), build_channel_id.clone());

    // Send the build command
    let build_command = json!({
        "command": "start_build",
        "fs_hash": fs_info.head_hash,
        "store_id": fs_info.store_id,
        "build_store_id": app_state.build_store_id
    });

    let build_command_bytes = match serde_json::to_vec(&build_command) {
        Ok(bytes) => bytes,
        Err(e) => {
            let error_msg = format!("Failed to serialize build command: {}", e);
            log(&error_msg);

            // Send error to frontend
            let error_msg = FrontendMessage::OperationCompleted {
                operation_id: operation_id.clone(),
                success: false,
                message: error_msg.clone(),
            };

            if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
                let _ = send_on_channel(frontend_channel_id, &msg_bytes);
            }

            // Clean up
            app_state.actor_channels.remove(&operation_id);
            app_state.active_operations.remove(&operation_id);

            return Err(error_msg);
        }
    };

    // Send the build command to the build actor
    if let Err(e) = send_on_channel(&build_channel_id, &build_command_bytes) {
        let error_msg = format!("Failed to send build command: {}", e);
        log(&error_msg);

        // Send error to frontend
        let error_msg = FrontendMessage::OperationCompleted {
            operation_id: operation_id.clone(),
            success: false,
            message: error_msg.clone(),
        };

        if let Ok(msg_bytes) = serde_json::to_vec(&error_msg) {
            let _ = send_on_channel(frontend_channel_id, &msg_bytes);
        }

        // Clean up
        app_state.actor_channels.remove(&operation_id);
        app_state.active_operations.remove(&operation_id);

        return Err(error_msg);
    }

    log(&format!("Sent build command to build actor"));

    // Send a progress update to the frontend
    let progress_msg = FrontendMessage::OperationProgress {
        operation_id: operation_id.clone(),
        description: "Build started, waiting for updates...".to_string(),
        percent_complete: Some(5.0),
    };

    if let Ok(msg_bytes) = serde_json::to_vec(&progress_msg) {
        let _ = send_on_channel(frontend_channel_id, &msg_bytes);
    }

    // The rest will happen asynchronously via channel messages
    Ok(())
}
