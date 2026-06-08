use std::io;
use clap::Parser;

mod args;
mod types;
mod monitor;
mod network;
mod utils;
mod stealth;

use args::Args;
use monitor::ProcessMonitor;
use crate::stealth::StealthManager;

fn main() -> io::Result<()> {
    let args = Args::parse();

    if args.stealth_config {
        let mut stealth_manager = StealthManager::new();
        if let Err(e) = stealth_manager.interactive_config() {
            eprintln!("Error configuring stealth settings:\n{}", e);
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Error configuring stealth settings"));
        }
        return Ok(());
    }

    let mut monitor = ProcessMonitor::new(args);
    monitor.run()
}