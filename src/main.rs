use std::io;
use clap::Parser;

mod args;
mod types;
mod monitor;
mod utils;

use args::Args;
use monitor::ProcessMonitor;

fn main() -> io::Result<()> {
    let args = Args::parse();
    let mut monitor = ProcessMonitor::new(args);
    monitor.run()
}