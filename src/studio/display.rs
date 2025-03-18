use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::time::Duration;

pub fn render_banner() -> String {
    let banner = r#"
  _______  _                 _               
 |__   __|| |               | |              
    | |   | |__    ___  ___ | |_  ___  _ __ 
    | |   | '_ \  / _ \/ __|| __|/ _ \| '__|
    | |   | | | ||  __/\__ \| |_|  __/| |   
    |_|   |_| |_| \___||___/ \__|\___||_|   
                 Studio CLI v0.1.0                 
    "#;
    
    banner.bright_blue().bold().to_string()
}

pub fn print_success(message: &str) {
    println!("{} {}", "SUCCESS:".green().bold(), message);
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "ERROR:".red().bold(), message);
}

pub fn print_warning(message: &str) {
    println!("{} {}", "WARNING:".yellow().bold(), message);
}

pub fn print_info(message: &str) {
    println!("{} {}", "INFO:".blue().bold(), message);
}

pub fn format_json(json_str: &str) -> String {
    match serde_json::from_str::<Value>(json_str) {
        Ok(json) => {
            match serde_json::to_string_pretty(&json) {
                Ok(formatted) => formatted,
                Err(_) => json_str.to_string(),
            }
        },
        Err(_) => json_str.to_string(),
    }
}

pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[
                "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"
            ])
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

pub fn print_session_status(manager_id: Option<&str>, actor_id: Option<&str>, build_status: &str) {
    println!("\n{}", "=== Theater Studio Session Status ===".blue().bold());
    println!("{} {}", "Manager Actor:".yellow(), manager_id.unwrap_or("Not started"));
    println!("{} {}", "Child Actor:".yellow(), actor_id.unwrap_or("Not started"));
    println!("{} {}", "Build Status:".yellow(), build_status);
    println!("{}\n", "=================================".blue().bold());
}

pub fn print_prompt() -> String {
    format!("{} ", "theater>".green().bold())
}
