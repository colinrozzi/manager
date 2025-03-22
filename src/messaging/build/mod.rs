mod parser;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub use parser::parse_build_message;

/// Message types sent by the build actor
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BuildActorMessage {
    Log {
        level: String,
        message: String,
    },
    
    Progress {
        status: String,
        description: String,
        percent_complete: Option<f32>,
    },
    
    CommandStarted {
        command: String,
        args: Vec<String>,
    },
    
    CommandOutput {
        stdout: String,
        stderr: String,
    },
    
    BuildComplete {
        success: bool,
        wasm_path: Option<String>,
        wasm_hash: Option<String>,
        error: Option<String>,
    },
    
    FileExtracted {
        path: String,
        size: usize,
    }
}

impl From<&BuildActorMessage> for String {
    fn from(message: &BuildActorMessage) -> Self {
        match message {
            BuildActorMessage::Log { .. } => "Log".to_string(),
            BuildActorMessage::Progress { .. } => "Progress".to_string(),
            BuildActorMessage::CommandStarted { .. } => "CommandStarted".to_string(),
            BuildActorMessage::CommandOutput { .. } => "CommandOutput".to_string(), 
            BuildActorMessage::BuildComplete { .. } => "BuildComplete".to_string(),
            BuildActorMessage::FileExtracted { .. } => "FileExtracted".to_string(),
        }
    }
}

impl BuildActorMessage {
    pub fn get_message(&self) -> String {
        match self {
            BuildActorMessage::Log { message, .. } => message.clone(),
            BuildActorMessage::Progress { description, .. } => description.clone(),
            BuildActorMessage::CommandStarted { command, args } => 
                format!("Running command: {} {}", command, args.join(" ")),
            BuildActorMessage::CommandOutput { stdout, stderr } => {
                if !stderr.is_empty() {
                    format!("Command output: {} (with errors: {})", stdout, stderr)
                } else {
                    format!("Command output: {}", stdout)
                }
            },
            BuildActorMessage::BuildComplete { success, error, .. } => {
                if *success {
                    "Build completed successfully".to_string()
                } else if let Some(err) = error {
                    format!("Build failed: {}", err)
                } else {
                    "Build failed".to_string()
                }
            },
            BuildActorMessage::FileExtracted { path, size } => {
                format!("Extracted file: {} ({} bytes)", path, size)
            }
        }
    }
}
