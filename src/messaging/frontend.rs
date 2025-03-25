use crate::state::{OperationSummary, OperationType};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Commands received from the frontend
#[derive(Debug, Serialize, Deserialize)]
pub enum FrontendCommand {
    StartActor,
    StopActor,
    BuildActor,
    ChangeRequest { description: String },
    GetStatus,
    Disconnect,
}

/// Messages to send to the frontend
#[derive(Debug, Serialize, Deserialize)]
pub enum FrontendMessage {
    // Status messages
    Status {
        child_running: bool,
        active_operations: Vec<OperationSummary>,
    },

    // Operation messages
    OperationStarted {
        operation_id: String,
        operation_type: OperationType,
        description: String,
    },
    OperationProgress {
        operation_id: String,
        description: String,
        percent_complete: Option<f32>,
    },
    OperationCompleted {
        operation_id: String,
        success: bool,
        message: String,
    },

    // Actor events (forwarded from child actors)
    BuildEvent {
        operation_id: String,
        event_type: String,
        message: String,
        details: Value,
    },
    ProgrammerEvent {
        operation_id: String,
        event_type: String,
        message: String,
        details: Value,
    },
    ChildStarted {
        child_id: String,
    },
    ChildStopped {
        child_id: String,
    },

    // Log messages
    Log {
        level: String,
        message: String,
    },

    // Error messages
    Error {
        code: String,
        message: String,
    },
}
