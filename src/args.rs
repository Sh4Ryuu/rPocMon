use clap::Parser;

#[derive(Parser)]
#[command(name = "rprocmon")]
#[command(about = "A Rust-based process monitor for security analysis")]
pub struct Args {
    /// Monitor interval in seconds
    #[arg(short, long, default_value = "2")]
    pub interval: u64,

    /// Save output to JSON file
    #[arg(short, long)]
    pub output: Option<String>,

    /// Filter by process name
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Show network connections
    #[arg(short, long)]
    pub network: bool,

    /// Alert on new processes
    #[arg(short, long)]
    pub alert: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}