use crate::messaging::ChannelType;
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
    pub channels: HashMap<String, ChannelType>,  // Maps channel_id -> channel type
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

// Removed the legacy Action enum as we've migrated to channel-only communication

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
            channels: HashMap::new(),
            active_operations: HashMap::new(),
        }
    }

    // Helper method to register a channel
    pub fn register_channel(&mut self, channel_id: String, channel_type: ChannelType) {
        self.channels
            .insert(channel_id.clone(), channel_type.clone());

        // Also update actor_channels for compatibility during transition
        match &channel_type {
            ChannelType::Build { operation_id } | ChannelType::Programmer { operation_id } => {
                self.actor_channels.insert(operation_id.clone(), channel_id);
            }
            _ => {}
        }
    }

    // Helper to get operation ID from channel ID
    pub fn get_operation_for_channel(&self, channel_id: &str) -> Option<String> {
        match self.channels.get(channel_id) {
            Some(ChannelType::Build { operation_id }) => Some(operation_id.clone()),
            Some(ChannelType::Programmer { operation_id }) => Some(operation_id.clone()),
            _ => None,
        }
    }

    // Helper to get channel type
    pub fn get_channel_type(&self, channel_id: &str) -> ChannelType {
        self.channels.get(channel_id).cloned().unwrap_or_else(|| {
            if self.frontend_channel_id.as_ref() == Some(&channel_id.to_string()) {
                ChannelType::Frontend
            } else {
                ChannelType::Unknown
            }
        })
    }
}

// Public summary of an operation for the frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct OperationSummary {
    pub operation_id: String,
    pub operation_type: OperationType,
    pub status: OperationStatus,
}

impl From<&OperationState> for OperationSummary {
    fn from(op_state: &OperationState) -> Self {
        Self {
            operation_id: op_state.operation_id.clone(),
            operation_type: op_state.operation_type.clone(),
            status: op_state.status.clone(),
        }
    }
}
