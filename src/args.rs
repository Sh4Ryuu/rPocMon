use clap::Parser;

#[derive(Clone, Debug, Default, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Json,
    Csv,
}

#[derive(Parser)]
#[command(name = "rprocmon")]
#[command(about = "A Rust-based process monitor for security analysis")]
pub struct Args {
    /// Monitor interval in seconds
    #[arg(short, long, default_value = "2")]
    pub interval: u64,

    /// Save output to a file on exit
    #[arg(short, long)]
    pub output: Option<String>,

    /// Filter by process name
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Show per-process network sockets (TCP/UDP)
    #[arg(short, long)]
    pub network: bool,

    /// Alert on new processes
    #[arg(short, long)]
    pub alert: bool,

    /// Verbose output (show full command line and exe path)
    #[arg(short, long)]
    pub verbose: bool,

    /// Focus on a single PID — shows extra detail and its network connections
    #[arg(short = 'p', long)]
    pub pid: Option<u32>,

    /// Output format when saving snapshots (json or csv)
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,

    /// Open interactive stealth configuration menu
    #[arg(long)]
    pub stealth_config: bool,
}
