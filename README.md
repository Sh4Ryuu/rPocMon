# RProcMon 🔍

A powerful, real-time process monitor written in Rust for security analysis and system monitoring.

## Features

- **Real-time Process Monitoring**: Track running processes with CPU and memory usage
- **Network Activity Monitoring**: Display active network connections and interfaces
- **Process Alerting**: Get notifications when new processes start
- **Data Export**: Save monitoring snapshots to JSON files
- **Process Filtering**: Filter processes by name for focused monitoring
- **Interactive Interface**: Clean terminal UI with color-coded CPU usage alerts
- **Security Analysis**: Built-in detection for potentially suspicious activities
- **Cross-platform**: Works on Windows, macOS, and Linux

## Installation

### Prerequisites
- Rust (install from [rustup.rs](https://rustup.rs/))

### Build from Source
```bash
cargo build --release
```

## Usage

### Basic Usage
```bash
# Start monitoring with default 2-second intervals
cargo run

# Monitor with custom interval
cargo run -- -i 5

# Show help
cargo run -- -h
```

### Command Line Options

| Option | Short        | Description                              |
|--------|--------------|------------------------------------------|
| `-i`   | `--interval` | Monitor interval in seconds (default: 2) |
| `-o`   | `--output`   | Save output to JSON file                 |
| `-f`   | `--filter`   | Filter by process name                   |
| `-n`   | `--network`  | Show network connections                 |
| `-a`   | `--alert`    | Alert on new processes                   |
| `-v`   | `--verbose`  | Verbose output with command details      |
| `-h`   | `--help`     | Print help information                   |

### Examples

```bash
# Monitor with alerts for new processes
cargo run -- -a

# Filter processes containing "chrome" with network monitoring
cargo run -- -f chrome -n

# Save monitoring data to file with verbose output
cargo run -- -o monitoring.json -v

# Monitor every 10 seconds with all features enabled
cargo run -- -i 10 -n -a -v
```

### Interactive Controls

While running, use these keyboard shortcuts:
- **`q`** - Quit the monitor
- **`s`** - Save current snapshot to JSON file
- **`c`** - Clear the screen

## Output Information

### Process Display
- **PID**: Process ID
- **NAME**: Process name (truncated to 25 chars)
- **CPU%**: Current CPU usage percentage
- **MEMORY(KB)**: Memory usage in kilobytes
- **PPID**: Parent Process ID
- **USER_ID**: User ID running the process
- **STATUS**: Current process status

### Color Coding
- **🔴 Red**: Processes using >50% CPU
- **🟡 Yellow**: Processes using 25-50% CPU
- **⚪ White**: Normal CPU usage
- **🟢 Green**: New process alerts

### System Overview
- Memory usage percentage and absolute values
- CPU count
- System uptime
- Total process count
- Monitor runtime duration

## JSON Export Format

Snapshots are saved in structured JSON format containing:
- Timestamp
- Process information (PID, name, CPU, memory, etc.)
- Network connections (if enabled)
- System information (memory, CPU count, uptime)

## Security Features

RProcMon includes built-in security analysis capabilities:
- High CPU usage detection
- Process monitoring from temporary directories
- Orphaned process detection
- Suspicious process name identification
- New process alerting

## Project Structure

```
src/
├── main.rs          # Entry point
├── args.rs          # Command line argument parsing
├── types.rs         # Data structures and types
├── monitor.rs       # Main monitoring logic
└── utils.rs         # Utility functions
```

## Dependencies

- **sysinfo**: System and process information
- **clap**: Command line argument parsing
- **chrono**: Date and time handling
- **serde**: Serialization for JSON export
- **crossterm**: Cross-platform terminal manipulation

## Performance

- Lightweight and efficient
- Minimal system resource usage
- Configurable refresh intervals
- Real-time updates without blocking

## Platform Support

- ✅ **Linux**: Full support
- ✅ **Windows**: Full support
- ✅ **macOS**: Full support
- ✅ **FreeBSD**: Basic support

## Contributing

Contributions are welcome! Please feel free to:
- Report bugs and issues
- Suggest new features
- Submit pull requests
- Improve documentation

## Acknowledgments

- Built with the amazing Rust ecosystem
- Inspired by traditional system monitoring tools

---
