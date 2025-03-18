use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Parser, Debug, Clone)]
#[command(name = "Theater Studio")]
#[command(version = "0.1.0")]
#[command(about = "An interactive CLI for the Theater Actor System", long_about = None)]
pub struct Args {
    /// Path to the Theater server executable
    #[arg(long, default_value = "theater")]
    pub theater_path: String,

    /// Port to connect to the Theater server
    #[arg(long, default_value_t = 8080)]
    pub port: u16,

    /// Host for the Theater server
    #[arg(long, default_value = "localhost")]
    pub host: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    // Session Commands
    Start,
    Stop,
    Status,
    
    // Code Commands
    Change(String),
    Build,
    
    // Actor Commands
    StartActor,
    StopActor,
    RestartActor,
    Logs,
    
    // Interaction Commands
    Message(String),
    State,
    Http { method: String, path: String, data: Option<String> },
    
    // Utility Commands
    Help,
    Clear,
    Exit,

    // Invalid command
    Invalid(String),
}

impl Command {
    pub fn from_input(input: &str) -> Self {
        let input = input.trim();

        // Split the input into command and arguments
        let mut parts = input.splitn(2, ' ');
        let command = parts.next().unwrap_or("").to_lowercase();
        let args = parts.next().unwrap_or("").trim();

        match command.as_str() {
            // Session Commands
            "start" => Command::Start,
            "stop" => Command::Stop,
            "status" => Command::Status,
            
            // Code Commands
            "change" => {
                if args.is_empty() {
                    Command::Invalid("Change command requires a description".to_string())
                } else {
                    Command::Change(args.to_string())
                }
            },
            "build" => Command::Build,
            
            // Actor Commands
            "start-actor" => Command::StartActor,
            "stop-actor" => Command::StopActor,
            "restart-actor" => Command::RestartActor,
            "logs" => Command::Logs,
            
            // Interaction Commands
            "message" => {
                if args.is_empty() {
                    Command::Invalid("Message command requires content".to_string())
                } else {
                    Command::Message(args.to_string())
                }
            },
            "state" => Command::State,
            "http" => {
                let http_parts: Vec<&str> = args.splitn(3, ' ').collect();
                if http_parts.len() < 2 {
                    Command::Invalid("HTTP command requires method and path".to_string())
                } else {
                    let method = http_parts[0].to_uppercase();
                    let path = http_parts[1].to_string();
                    let data = if http_parts.len() > 2 {
                        Some(http_parts[2].to_string())
                    } else {
                        None
                    };
                    Command::Http { method, path, data }
                }
            },
            
            // Utility Commands
            "help" => Command::Help,
            "clear" => Command::Clear,
            "exit" | "quit" => Command::Exit,
            
            // Empty or invalid command
            "" => Command::Invalid("Empty command".to_string()),
            _ => Command::Invalid(format!("Unknown command: {}", command)),
        }
    }

    pub fn get_help_text() -> String {
        r#"
AVAILABLE COMMANDS:

Session Commands:
  start               Start a new development session
  stop                Stop the current development session
  status              Show the status of the current session

Code Commands:
  change <desc>       Submit code changes with description
  build               Build the current code into a WebAssembly actor

Actor Commands:
  start-actor         Start the built actor
  stop-actor          Stop the running actor
  restart-actor       Restart the running actor
  logs                Show actor logs

Interaction Commands:
  message <content>   Send a message to the running actor
  state               Display the actor's current state
  http <method> <path> [data]   Send HTTP request to actor

Utility Commands:
  help                Display this help information
  clear               Clear the terminal screen
  exit                Exit the Theater Studio CLI
"#.to_string()
    }
}

#[derive(Error, Debug)]
pub enum CommandError {
    #[error("Session error: {0}")]
    Session(String),

    #[error("Actor error: {0}")]
    Actor(String),

    #[error("Communication error: {0}")]
    Communication(String),

    #[error("Invalid command: {0}")]
    Invalid(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
