pub mod build;
pub mod frontend;
pub mod handlers;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelType {
    Frontend,
    Build { operation_id: String },
    Programmer { operation_id: String },
    Unknown,
}
