use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::thread;
use uuid::Uuid;

use crate::commands::CommandError;
use crate::display;

#[derive(Serialize, Deserialize, Debug)]
pub enum Action {
    Start,
    Stop,
    Build,
    Change(String),
}

#[derive(Debug)]
pub struct SessionInfo {
    pub theater_path: String,
    pub host: String,
    pub port: u16,
    pub manager_actor_id: Option<String>,
    pub child_actor_id: Option<String>,
    pub build_status: String,
    pub running: bool,
}

pub struct Session {
    pub info: SessionInfo,
    pub server_process: Option<std::process::Child>,
}

impl Session {
    pub fn new(theater_path: String, host: String, port: u16) -> Self {
        Self {
            info: SessionInfo {
                theater_path,
                host,
                port,
                manager_actor_id: None,
                child_actor_id: None,
                build_status: "Not built".to_string(),
                running: false,
            },
            server_process: None,
        }
    }

    pub fn start(&mut self) -> Result<(), CommandError> {
        if self.info.running {
            return Err(CommandError::Session("Session already running".to_string()));
        }

        let spinner = display::create_spinner("Starting Theater server...");
        
        // Start the Theater server as a child process
        let process = Command::new(&self.info.theater_path)
            .arg("server")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match process {
            Ok(server_process) => {
                self.server_process = Some(server_process);
                // Give the server a moment to start
                thread::sleep(Duration::from_secs(1));
                spinner.finish_with_message("Theater server started");
            }
            Err(e) => {
                spinner.finish_with_message(format!("Failed to start Theater server: {}", e));
                return Err(CommandError::Session(format!("Failed to start server: {}", e)));
            }
        }

        // Now start the manager actor
        let spinner = display::create_spinner("Starting manager actor...");
        
        // Generate a unique ID for this session
        let _session_id = Uuid::new_v4().to_string();

        let output = Command::new(&self.info.theater_path)
            .arg("start")
            .arg("/users/colinrozzi/work/actors/manager/manifest.toml")
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    // Extract the actor ID from the output (assuming it's in the format "Actor started with ID: <id>")
                    let actor_id = if let Some(id_line) = output_str.lines().find(|line| line.contains("Actor started with ID:")) {
                        if let Some(id) = id_line.split("ID:").nth(1) {
                            Some(id.trim().to_string())
                        } else {
                            None
                        }
                    } else {
                        // Fallback: try to find any UUID in the output
                        let uuid_pattern = r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
                        if let Some(captures) = regex::Regex::new(uuid_pattern).unwrap().captures(&output_str) {
                            Some(captures.get(0).unwrap().as_str().to_string())
                        } else {
                            None
                        }
                    };

                    if let Some(id) = actor_id {
                    self.info.manager_actor_id = Some(id);
                    self.info.running = true;
                    spinner.finish_with_message(format!("Manager actor started with ID: {}", self.info.manager_actor_id.as_ref().unwrap()));
                    } else {
                            spinner.finish_with_message("Manager actor started but couldn't extract ID");
                            display::print_warning("Could not extract manager actor ID from output");
                            // Since we don't have the ID, we'll use a placeholder
                            self.info.manager_actor_id = Some("unknown".to_string());
                            self.info.running = true;
                    }
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    spinner.finish_with_message(format!("Failed to start manager actor: {}", error));
                    return Err(CommandError::Session(format!("Failed to start manager actor: {}", error)));
                }
            }
            Err(e) => {
                spinner.finish_with_message(format!("Failed to start manager actor: {}", e));
                return Err(CommandError::Session(format!("Failed to start manager actor: {}", e)));
            }
        }

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), CommandError> {
        if !self.info.running {
            return Err(CommandError::Session("No session running".to_string()));
        }

        let spinner = display::create_spinner("Stopping actors...");

        // First try to stop the child actor if it's running
        if let Some(_child_id) = &self.info.child_actor_id {
            let _ = self.send_action(Action::Stop);
            self.info.child_actor_id = None;
        }

        // Then stop the manager actor
        if let Some(manager_id) = &self.info.manager_actor_id {
            let output = Command::new(&self.info.theater_path)
                .arg("stop")
                .arg(manager_id)
                .output();

            match output {
                Ok(output) => {
                    if !output.status.success() {
                        let error = String::from_utf8_lossy(&output.stderr);
                        let error_msg = format!("Failed to stop manager actor: {}", error);
                        spinner.finish_with_message(error_msg);
                        display::print_warning(&format!("Failed to stop manager actor: {}", error));
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to stop manager actor: {}", e);
                    spinner.finish_with_message(error_msg);
                    display::print_warning(&format!("Failed to stop manager actor: {}", e));
                }
            }
        }

        // Finally stop the server process
        if let Some(mut process) = self.server_process.take() {
            match process.kill() {
                Ok(_) => {
                    let _ = process.wait();
                    spinner.finish_with_message("Theater server stopped");
                }
                Err(e) => {
                    let error_msg = format!("Failed to stop Theater server: {}", e);
                    spinner.finish_with_message(error_msg);
                    display::print_warning(&format!("Failed to stop Theater server: {}", e));
                }
            }
        }

        self.info.running = false;
        self.info.manager_actor_id = None;
        self.info.child_actor_id = None;

        Ok(())
    }

    pub fn build(&mut self) -> Result<(), CommandError> {
        if !self.info.running {
            return Err(CommandError::Session("No session running".to_string()));
        }

        let spinner = display::create_spinner("Building actor...");

        match self.send_action(Action::Build) {
            Ok(response) => {
                spinner.finish_with_message("Build complete");
                self.info.build_status = "Built".to_string();
                display::print_success(&format!("Build response: {}", response));
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Build failed: {}", e);
                spinner.finish_with_message(error_msg);
                self.info.build_status = "Failed".to_string();
                Err(e)
            }
        }
    }

    pub fn submit_change(&mut self, description: &str) -> Result<(), CommandError> {
        if !self.info.running {
            return Err(CommandError::Session("No session running".to_string()));
        }

        let spinner = display::create_spinner("Submitting code change...");

        match self.send_action(Action::Change(description.to_string())) {
            Ok(response) => {
                spinner.finish_with_message("Change submitted");
                display::print_success(&format!("Response: {}", response));
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to submit change: {}", e);
                spinner.finish_with_message(error_msg);
                Err(e)
            }
        }
    }

    pub fn start_actor(&mut self) -> Result<(), CommandError> {
        if !self.info.running {
            return Err(CommandError::Session("No session running".to_string()));
        }

        let spinner = display::create_spinner("Starting actor...");

        match self.send_action(Action::Start) {
            Ok(response) => {
                spinner.finish_with_message("Actor started");
                // Assuming the actor ID is in the response
                self.info.child_actor_id = Some("running".to_string());
                display::print_success(&format!("Response: {}", response));
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to start actor: {}", e);
                spinner.finish_with_message(error_msg);
                Err(e)
            }
        }
    }

    pub fn stop_actor(&mut self) -> Result<(), CommandError> {
        if !self.info.running {
            return Err(CommandError::Session("No session running".to_string()));
        }

        if self.info.child_actor_id.is_none() {
            return Err(CommandError::Actor("No actor running".to_string()));
        }

        let spinner = display::create_spinner("Stopping actor...");

        match self.send_action(Action::Stop) {
            Ok(response) => {
                spinner.finish_with_message("Actor stopped");
                self.info.child_actor_id = None;
                display::print_success(&format!("Response: {}", response));
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to stop actor: {}", e);
                spinner.finish_with_message(error_msg);
                Err(e)
            }
        }
    }

    pub fn restart_actor(&mut self) -> Result<(), CommandError> {
        if !self.info.running {
            return Err(CommandError::Session("No session running".to_string()));
        }

        // First stop the actor
        let _ = self.stop_actor();
        
        // Then start it again
        self.start_actor()
    }

    pub fn get_logs(&self) -> Result<String, CommandError> {
        if !self.info.running {
            return Err(CommandError::Session("No session running".to_string()));
        }

        // In a real implementation, this would fetch logs from the actor
        // For now, we'll return a placeholder
        Ok("No logs available in this preview version".to_string())
    }

    pub fn send_message(&self, message: &str) -> Result<String, CommandError> {
        if !self.info.running {
            return Err(CommandError::Session("No session running".to_string()));
        }

        if self.info.child_actor_id.is_none() {
            return Err(CommandError::Actor("No actor running".to_string()));
        }

        // In a real implementation, this would send a message to the actor
        // For now, we'll return a placeholder
        Ok(format!("Message sent: {}", message))
    }

    pub fn get_state(&self) -> Result<String, CommandError> {
        if !self.info.running {
            return Err(CommandError::Session("No session running".to_string()));
        }

        // In a real implementation, this would fetch the actor's state
        // For now, we'll return a placeholder
        Ok("Actor state: running".to_string())
    }

    pub fn send_http_request(&self, method: &str, path: &str, _data: Option<&str>) -> Result<String, CommandError> {
        if !self.info.running {
            return Err(CommandError::Session("No session running".to_string()));
        }

        if self.info.child_actor_id.is_none() {
            return Err(CommandError::Actor("No actor running".to_string()));
        }

        // In a real implementation, this would send an HTTP request to the actor
        // For now, we'll return a placeholder
        Ok(format!("HTTP {} request sent to {}", method, path))
    }

    // Helper function to send an action to the manager actor
    fn send_action(&self, action: Action) -> Result<String, CommandError> {
        if let Some(manager_id) = &self.info.manager_actor_id {
            // Serialize the action to JSON
            let action_json = serde_json::to_string(&action)
                .map_err(|e| CommandError::Communication(format!("Failed to serialize action: {}", e)))?;

            // Create a command to send the message to the actor
            let mut child = Command::new(&self.info.theater_path)
                .arg("message")
                .arg(manager_id)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| CommandError::Communication(format!("Failed to spawn command: {}", e)))?;

            // Write the action JSON to the stdin of the command
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(action_json.as_bytes())
                    .map_err(|e| CommandError::Communication(format!("Failed to write to stdin: {}", e)))?;
            } else {
                return Err(CommandError::Communication("Failed to open stdin".to_string()));
            }

            // Wait for the command to complete and get its output
            let output = child.wait_with_output()
                .map_err(|e| CommandError::Communication(format!("Failed to wait for command: {}", e)))?;

            if output.status.success() {
                let response = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(response)
            } else {
                let error = String::from_utf8_lossy(&output.stderr).to_string();
                Err(CommandError::Communication(format!("Command failed: {}", error)))
            }
        } else {
            Err(CommandError::Session("No manager actor running".to_string()))
        }
    }
}
