use rustyline::error::ReadlineError;
use rustyline::{Editor, DefaultHelper};
use std::io::{self, Write};
use colored::Colorize;

use crate::commands::{Args, Command, CommandError};
use crate::display;
use crate::session::Session;

pub struct Repl {
    editor: Editor<DefaultHelper>,
    session: Session,
}

impl Repl {
    pub fn new(args: Args) -> Self {
        let editor = Editor::<DefaultHelper>::new().expect("Failed to create line editor");
        let session = Session::new(
            args.theater_path.clone(),
            args.host.clone(),
            args.port,
        );

        Self { editor, session }
    }

    pub fn run(&mut self) -> Result<(), CommandError> {
        println!("{}", "Welcome to Theater Studio CLI!".bright_green());
        println!("Type {} for usage information.", "help".bright_blue());
        
        loop {
            // Display the prompt and get user input
            let prompt = display::print_prompt();
            let readline = self.editor.readline(&prompt);

            match readline {
                Ok(line) => {
                    // Add the line to history
                    let _ = self.editor.add_history_entry(line.as_str());

                    // Parse the command
                    let command = Command::from_input(&line);

                    // Execute the command
                    match self.execute_command(command) {
                        Ok(should_exit) => {
                            if should_exit {
                                break;
                            }
                        }
                        Err(err) => {
                            display::print_error(&format!("{}", err));
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Press Ctrl-D or type 'exit' to exit");
                }
                Err(ReadlineError::Eof) => {
                    println!("Exiting...");
                    break;
                }
                Err(err) => {
                    display::print_error(&format!("Error: {}", err));
                    break;
                }
            }

            // Flush stdout to ensure prompt and output are displayed correctly
            let _ = io::stdout().flush();
        }

        // Make sure to clean up any running sessions
        if self.session.info.running {
            let _ = self.session.stop();
        }

        Ok(())
    }

    fn execute_command(&mut self, command: Command) -> Result<bool, CommandError> {
        match command {
            // Session Commands
            Command::Start => {
                self.session.start()?;
                Ok(false)
            }
            Command::Stop => {
                self.session.stop()?;
                Ok(false)
            }
            Command::Status => {
                display::print_session_status(
                    self.session.info.manager_actor_id.as_deref(),
                    self.session.info.child_actor_id.as_deref(),
                    &self.session.info.build_status,
                );
                Ok(false)
            }
            
            // Code Commands
            Command::Change(description) => {
                self.session.submit_change(&description)?;
                Ok(false)
            }
            Command::Build => {
                self.session.build()?;
                Ok(false)
            }
            
            // Actor Commands
            Command::StartActor => {
                self.session.start_actor()?;
                Ok(false)
            }
            Command::StopActor => {
                self.session.stop_actor()?;
                Ok(false)
            }
            Command::RestartActor => {
                self.session.restart_actor()?;
                Ok(false)
            }
            Command::Logs => {
                let logs = self.session.get_logs()?;
                println!("\n=== Actor Logs ===");
                println!("{}", logs);
                println!("=================\n");
                Ok(false)
            }
            
            // Interaction Commands
            Command::Message(content) => {
                let response = self.session.send_message(&content)?;
                println!("Response: {}", response);
                Ok(false)
            }
            Command::State => {
                let state = self.session.get_state()?;
                println!("State: {}", state);
                Ok(false)
            }
            Command::Http { method, path, data } => {
                let response = self.session.send_http_request(
                    &method,
                    &path,
                    data.as_deref(),
                )?;
                println!("Response: {}", response);
                Ok(false)
            }
            
            // Utility Commands
            Command::Help => {
                println!("{}", Command::get_help_text());
                Ok(false)
            }
            Command::Clear => {
                // Clear the screen using ANSI escape code
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush().unwrap();
                Ok(false)
            }
            Command::Exit => {
                // Clean up and exit
                if self.session.info.running {
                    display::print_info("Stopping active session before exit...");
                    let _ = self.session.stop();
                }
                Ok(true)
            }
            
            // Invalid command
            Command::Invalid(message) => {
                display::print_error(&format!("Invalid command: {}", message));
                Ok(false)
            }
        }
    }
}
