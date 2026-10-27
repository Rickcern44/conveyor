//! Conveyor's CLI application boundary.
//!
//! `cli` owns user-facing syntax and `commands` owns command dispatch and
//! handlers. Domain policy remains in `conveyor-core`.

mod cli;
mod commands;

use clap::Parser;

pub fn main() {
    let cli = cli::Cli::parse();
    match commands::execute(cli, &current_directory()) {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("conveyor: {error}");
            std::process::exit(1);
        }
    }
}

fn current_directory() -> std::path::PathBuf {
    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
}
