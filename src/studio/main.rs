use clap::Parser;
use colored::*;
use std::process;

mod commands;
mod display;
mod repl;
mod session;

use commands::Args;
use repl::Repl;

fn main() {
    println!("{}", display::render_banner());

    // Parse command line arguments
    let args = Args::parse();

    // Set up Ctrl+C handler
    ctrlc::set_handler(|| {
        println!("\n{}", "Exiting Theater Studio...".yellow());
        process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    // Create and run the REPL
    let mut repl = Repl::new(args);

    match repl.run() {
        Ok(_) => {
            println!("{}", "Thank you for using Theater Studio!".green());
        }
        Err(err) => {
            eprintln!("{} {}", "Error:".red().bold(), err);
            process::exit(1);
        }
    }
}
