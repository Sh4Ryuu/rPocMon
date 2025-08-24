use std::io;
use clap::Parser;
#[warn(unused_variables)]
mod args;
mod types;
mod monitor;
mod utils;
mod stealth;

use args::Args;
use monitor::ProcessMonitor;
use crate::stealth::StealthManager;

fn main() -> io::Result<()> {
    let args = Args::parse();
    if let Some(filter) = &args.filter {
        let mut stealth_manager = StealthManager::new();
        if let Err(e) = stealth_manager.interactive_config(){
            eprintln!("Error configuring stealth settings : \n{}", e);
            Err(io::Error::new(io::ErrorKind::InvalidInput, "Error configuring stealth settings"))?;
        }
        return Ok(());
    }
    let mut monitor = ProcessMonitor::new(args);
    monitor.run()
}