# RProcMon

A real-time, terminal-based process monitor written in Rust, built for security analysis and system inspection.

> Still under active development.

---

## Features

| Feature | Description |
|---------|-------------|
| **Real-time process monitoring** | CPU%, memory, PID, PPID, UID, status — sorted by CPU, refreshed every N seconds |
| **Per-process network sockets** | True `local:port → remote:port` per PID (not just interface byte counts) |
| **Per-core CPU breakdown** | Live usage for each core, color-coded, always visible in the header |
| **Process kill from TUI** | Arrow-key selection + `k`/`K` for SIGTERM / SIGKILL without leaving the monitor |
| **PID focus mode** | Drill into one process — memory %, open FDs, threads, CWD, its own sockets |
| **Memory color coding** | High-memory processes highlighted independently from CPU coloring |
| **JSON & CSV export** | Full structured JSON or flat CSV; snapshot on demand or on exit |
| **New process alerting** | Real-time notification when a process spawns |
| **Name filtering** | Show only processes matching a substring |
| **Process stealth** | Hide or rename processes by name or PID for operational use |
| **Responsive input** | Keys are processed throughout the full refresh interval — no blocked sleep |
| **Cross-platform** | Linux and Windows (network layer is platform-specific; see table below) |

---

## Platform Support

| Capability | Linux | Windows | macOS |
|------------|:-----:|:-------:|:-----:|
| Process monitoring | ✓ | ✓ | ✓ |
| Network connections | ✓ `/proc/net` | ✓ `netstat -ano` | — |
| Load average | ✓ | N/A | ✓ |
| PID focus extras (FD/CWD/threads) | ✓ | — | — |
| Process kill | ✓ | ✓ | ✓ |
| CSV / JSON export | ✓ | ✓ | ✓ |

> **Windows note:** `--network` calls `netstat -ano` internally. Run as Administrator for full socket visibility.

---

## Quick Start

```bash
# Build release binary
cargo build --release
# Binary at: ./target/release/rPocMon

# Run with defaults (2s interval, top 20 processes by CPU)
./target/release/rPocMon

# Or run via cargo during development
cargo run

# All flags
cargo run -- --help
```

---

## Command Line Options

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--interval` | `-i` | `2` | Refresh interval in seconds |
| `--output` | `-o` | — | File path to write all snapshots on exit |
| `--format` | | `json` | Output format: `json` or `csv` |
| `--filter` | `-f` | — | Show only processes whose name contains this string |
| `--pid` | `-p` | — | Focus mode: track a single PID in detail |
| `--network` | `-n` | off | Show per-process TCP/UDP sockets |
| `--alert` | `-a` | off | Print an alert when a new process appears |
| `--verbose` | `-v` | off | Show full command line and exe path per process |
| `--stealth-config` | | — | Open the interactive stealth configuration menu |

---

## Interactive Controls

| Key | Action |
|-----|--------|
| `q` | Quit |
| `↑` / `↓` | Move selection cursor through the process list |
| `k` | Send **SIGTERM** to selected process |
| `K` | Send **SIGKILL** to selected process |
| `s` | Save the current snapshot immediately (format follows `--format`) |
| `h` | Open stealth configuration menu |

---

## Usage Examples

```bash
# Basic monitor — refresh every 2s
cargo run

# Faster refresh with new-process alerts
cargo run -- -i 1 -a

# Real network sockets per process
cargo run -- -n

# Focus on one suspicious PID (shows FDs, threads, CWD, connections)
cargo run -- -p 1337

# Filter to processes named "nginx" and show their sockets
cargo run -- -f nginx -n

# Verbose output — full command lines and exe paths
cargo run -- -v

# Save all collected snapshots to JSON on exit
cargo run -- -o session.json

# Save as CSV instead
cargo run -- -o session.csv --format csv

# Full mode — all features, 5s interval
cargo run -- -i 5 -n -a -v -o session.json

# Configure which processes to hide/rename
cargo run -- --stealth-config
```

---

## Display Layout

```
RProcMon [2026-05-17 21:03:12]  Hidden: 0 | Renamed: 0 | Load: 0.45
Memory: 42.3% (6821/16106 MB) | CPUs: 8 | Uptime: 3600s | Monitor: 12s | Procs: 214
Keys: q=quit  s=save  k=SIGTERM  K=SIGKILL  UP/DOWN=select  h=stealth
Cores:      0: 12.5%   1:  8.1%   2: 45.3%   3:  2.0%   4:  0.0%   5:78.2%   6: 15.4%   7:  3.1%

-------------------------------------------------------------------------------------------------------
  PID     NAME                      CPU%     MEM(KB)      PPID     UID      STATUS
-------------------------------------------------------------------------------------------------------
> 4321    firefox                   45.2     512340       1        1000     Sleeping
  1234    nginx                      2.1      12480       1        33       Sleeping
  ...
```

### Process Table Columns

| Column | Description |
|--------|-------------|
| PID | Process ID |
| NAME | Process name (truncated to 25 characters) |
| CPU% | CPU usage this interval |
| MEM(KB) | Resident memory in kilobytes |
| PPID | Parent process ID |
| UID | User ID of the process owner |
| STATUS | Kernel-reported process state |

### Color Legend

**Process rows:**

| Color | Meaning |
|-------|---------|
| **Cyan** (highlight) | Currently selected process |
| **Red** | CPU > 50% |
| **Yellow** | CPU 25–50% |
| **Magenta** | Memory > 10% of total RAM (CPU is normal) |
| **Blue** | Memory 5–10% of total RAM (CPU is normal) |

**Network connections:**

| Color | State |
|-------|-------|
| **Green** | ESTABLISHED |
| **Cyan** | LISTEN / LISTENING |
| White | Other (SYN_*, TIME_WAIT, etc.) |

**Per-core CPU:**

| Color | Usage |
|-------|-------|
| **Red** | > 80% |
| **Yellow** | > 50% |
| White | Normal |

---

## Network View (`--network`)

Enabled with `-n`. Shows up to 15 sockets, sorted ESTABLISHED → LISTEN → other.

**Linux** — reads `/proc/net/tcp` and `/proc/net/udp`, resolves socket inodes to PIDs via `/proc/<pid>/fd/`. No external tools needed.

**Windows** — runs `netstat -ano`, maps PIDs to process names from sysinfo. Requires Administrator for full visibility.

```
Network [42 sockets, showing 15]
-------------------------------------------------------------------------------------------------------
PROTO PROCESS(PID)         LOCAL                    REMOTE                   STATE
-------------------------------------------------------------------------------------------------------
TCP   firefox(4321)        192.168.1.5:54321        52.96.51.23:443          ESTABLISHED
TCP   nginx(1234)          0.0.0.0:80               0.0.0.0:0                LISTEN
...
```

---

## PID Focus Mode (`--pid <pid>`)

Replaces the process table with a detailed panel for one process. Network connections are always shown in focus mode, even without `--network`.

```
===================================================================================================
  PID Focus: 4321 (firefox)
---------------------------------------------------------------------------------------------------
  Status   : Sleeping
  CPU      : 45.20%
  Memory   : 512340 KB  (3.18% of total RAM)
  Parent   : 1
  UID      : 1000
  Exe      : /usr/lib/firefox/firefox
  Command  : /usr/lib/firefox/firefox --new-window
  CWD      : /home/user                          ← Linux only
  Open FDs : 87                                  ← Linux only
  Threads  : 42                                  ← Linux only

  PROTO  LOCAL                    REMOTE                   STATE
  -----------------------------------------------------------------------
  TCP    192.168.1.5:54321        52.96.51.23:443          ESTABLISHED
===================================================================================================
```

> The target PID bypasses the stealth filter so it is always visible in focus mode, even if listed in `hidden_processes`.

---

## Stealth Configuration

Hide or rename processes from the monitor view — useful during engagements when you want to conceal your own tooling.

```bash
# Open from CLI before starting
cargo run -- --stealth-config

# Or press 'h' while the monitor is running
```

Menu options:
- List all running processes (by name or PID)
- Hide processes by name (substring match)
- Hide processes by PID
- Rename processes (display a different name)
- Show / clear current configuration

Settings persist to `stealth_config.json` in the working directory:

```json
{
  "hidden_processes": ["wireshark", "burpsuite"],
  "renamed_processes": {
    "rprocmon": "system_monitor"
  },
  "hidden_pids": [4321]
}
```

---

## Export Formats

### JSON (default)

Full structured output per snapshot: processes, network connections, per-core CPU, system info. Capped at 500 snapshots in memory to bound RAM usage.

```bash
cargo run -- -o session.json
```

### CSV (`--format csv`)

Flat process table, one row per process per interval. Network connection data is JSON-only.

```
timestamp,pid,name,cpu_pct,memory_kb,ppid,uid,status,exe_path,cmd
2026-05-17T21:03:12,4321,firefox,45.20,512340,1,1000,Sleeping,/usr/lib/firefox/firefox,...
```

```bash
cargo run -- -o session.csv --format csv
```

**`s` key** — writes the most recent snapshot immediately (useful during a live session). **`--output`** — writes all accumulated snapshots on clean exit.

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `sysinfo` | Cross-platform process info and signal delivery |
| `clap` | CLI argument parsing |
| `chrono` | Timestamps and date formatting |
| `serde` + `serde_json` | JSON serialization |
| `crossterm` | Cross-platform terminal control (raw mode, colors, cursor) |

---

## Contributing

Contributions are welcome:
- Bug reports and feature requests via issues
- Pull requests for fixes, new features, or documentation
- Testing on Windows and macOS is especially appreciated

## Acknowledgments

Built with the Rust ecosystem. Inspired by `top`, `htop`, `ss`, and similar Unix monitoring tools.
