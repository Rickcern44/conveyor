//! Conveyor's CLI application boundary.
//!
//! `cli` owns user-facing syntax, `commands` dispatches parsed requests, and
//! `legacy` contains the workflow handlers while they are migrated one command
//! group at a time. Domain policy remains in `conveyor-core`.

mod cli;
mod commands;
mod legacy;

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
