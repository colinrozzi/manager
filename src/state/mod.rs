use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct InitData {
    pub build_store_id: Option<String>,
    pub runtime_content_fs_actor_id: String,
    pub anthropic_api_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub child_id: Option<String>,
    pub programmer_actor_id: String,
    pub runtime_content_fs_actor_id: String,
    pub build_store_id: String,
    
    // Channel management
    pub frontend_channel_id: Option<String>,
    pub actor_channels: HashMap<String, String>, // Maps operation_id -> channel_id
    pub active_operations: HashMap<String, OperationState>, // Maps operation_id -> operation state
}

// Structure to track operations
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OperationState {
    pub operation_id: String,
    pub operation_type: OperationType,
    pub actor_id: String,
    pub channel_id: Option<String>,
    pub status: OperationStatus,
    pub start_time: u64,
    pub end_time: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum OperationType {
    Build,
    Change,
    Start,
    Stop,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Structure to hold build result information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub wasm_path: Option<String>,
    pub wasm_hash: Option<String>,
    pub build_logs: Vec<String>,
    pub error: Option<String>,
}

/// Result of a get info operation
#[derive(Serialize, Deserialize, Debug)]
pub struct InfoResult {
    pub head_hash: String,
    pub store_id: String,
}

// Legacy action enum for request/response API
#[derive(Serialize, Deserialize)]
pub enum Action {
    Start,
    Stop,
    Build,
    Change(String),
}

// Helper to create an initial AppState
impl AppState {
    pub fn new(
        runtime_content_fs_actor_id: String,
        build_store_id: String,
        programmer_actor_id: String,
    ) -> Self {
        Self {
            child_id: None,
            programmer_actor_id,
            runtime_content_fs_actor_id,
            build_store_id,
            frontend_channel_id: None,
            actor_channels: HashMap::new(),
            active_operations: HashMap::new(),
        }
    }
}

// Public summary of an operation for the frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationSummary {
    pub operation_id: String,
    pub operation_type: OperationType,
    pub status: OperationStatus,
    pub start_time: u64,
}

impl From<&OperationState> for OperationSummary {
    fn from(op_state: &OperationState) -> Self {
        Self {
            operation_id: op_state.operation_id.clone(),
            operation_type: op_state.operation_type.clone(),
            status: op_state.status.clone(),
            start_time: op_state.start_time,
        }
    }
}
