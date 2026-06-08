use sysinfo::{System, Signal};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::io::{self, stdout, Write};
use chrono::Local;
use crossterm::{
    execute,
    terminal::{Clear, ClearType, enable_raw_mode, disable_raw_mode},
    cursor::{MoveTo, Hide, Show},
    style::{Color, SetForegroundColor, SetBackgroundColor, ResetColor},
    event::{self, Event, KeyCode},
};

use crate::args::{Args, OutputFormat};
use crate::types::{ProcessInfo, NetworkConnection, MonitorSnapshot, SystemSnapshot};
use crate::utils::truncate_string;
use crate::stealth::StealthManager;

const MAX_DISPLAY: usize = 20;
const MAX_SNAPSHOTS: usize = 500;

// In raw mode \n moves the cursor down without a CR, so every line needs \r\n.
macro_rules! pln {
    () => { let _ = write!(stdout(), "\r\n"); };
    ($($arg:tt)*) => { let _ = write!(stdout(), "{}\r\n", format!($($arg)*)); };
}

pub struct ProcessMonitor {
    system: System,
    previous_processes: HashMap<u32, ProcessInfo>,
    args: Args,
    start_time: Instant,
    snapshots: Vec<MonitorSnapshot>,
    stealth_manager: StealthManager,
    selected_index: usize,
    display_list: Vec<ProcessInfo>,
    kill_status: Option<String>,
}

impl ProcessMonitor {
    pub fn new(args: Args) -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self {
            system,
            previous_processes: HashMap::new(),
            args,
            start_time: Instant::now(),
            snapshots: Vec::new(),
            stealth_manager: StealthManager::new(),
            selected_index: 0,
            display_list: Vec::new(),
            kill_status: None,
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        let result = self.run_loop();
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), Show);
        result
    }

    fn run_loop(&mut self) -> io::Result<()> {
        execute!(stdout(), Hide)?;

        'main: loop {
            execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
            self.system.refresh_all();

            let snapshot = self.collect_snapshot();

            // Header + CPU cores are always shown
            self.display_header(&snapshot);
            self.display_cpu_cores(&snapshot);

            if self.args.pid.is_some() {
                self.display_pid_focus(&snapshot);
            } else {
                self.display_process_table(&snapshot);
                if self.args.network {
                    self.display_network_connections(&snapshot);
                }
            }

            if let Some(status) = self.kill_status.take() {
                execute!(stdout(), SetForegroundColor(Color::Green)).unwrap();
                pln!("{}", status);
                execute!(stdout(), ResetColor).unwrap();
            }

            self.check_for_new_processes(&snapshot);

            self.snapshots.push(snapshot);
            if self.snapshots.len() > MAX_SNAPSHOTS {
                self.snapshots.drain(..self.snapshots.len() - MAX_SNAPSHOTS);
            }

            stdout().flush()?;

            // Poll for key events throughout the full interval — no blocking sleep
            let deadline = Instant::now() + Duration::from_secs(self.args.interval);
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                if !event::poll(remaining.min(Duration::from_millis(50)))? {
                    continue;
                }
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break 'main,
                        KeyCode::Char('s') => self.save_snapshot()?,
                        KeyCode::Up => {
                            self.selected_index = self.selected_index.saturating_sub(1);
                            break;
                        }
                        KeyCode::Down => {
                            if !self.display_list.is_empty() {
                                self.selected_index =
                                    (self.selected_index + 1).min(self.display_list.len() - 1);
                            }
                            break;
                        }
                        KeyCode::Char('k') => {
                            self.kill_selected(false);
                            break;
                        }
                        KeyCode::Char('K') => {
                            self.kill_selected(true);
                            break;
                        }
                        KeyCode::Char('h') => {
                            disable_raw_mode()?;
                            execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0), Show)?;
                            if let Err(e) = self.stealth_manager.interactive_config() {
                                println!("Error: {}", e);
                            }
                            enable_raw_mode()?;
                            execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0), Hide)?;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(path) = self.args.output.clone() {
            self.write_output(&path)?;
        }

        Ok(())
    }

    // ── Data collection ───────────────────────────────────────────────────────────────────────

    fn collect_snapshot(&mut self) -> MonitorSnapshot {
        let mut processes = Vec::new();
        let mut pid_names: Vec<(u32, String)> = Vec::new();

        let need_network = self.args.network || self.args.pid.is_some();

        for (pid, process) in self.system.processes() {
            let original_name = process.name().to_string_lossy().to_string();

            // Collect all PIDs unfiltered for the socket inode map
            if need_network {
                pid_names.push((pid.as_u32(), original_name.clone()));
            }

            // --pid bypasses the stealth filter so the target is always visible
            let is_focus_target = self.args.pid.map_or(false, |p| p == pid.as_u32());

            if !is_focus_target
                && (self.stealth_manager.is_process_hidden(&original_name)
                    || self.stealth_manager.is_pid_hidden(pid.as_u32()))
            {
                continue;
            }

            let display_name = self.stealth_manager.get_display_name(&original_name);

            let process_info = ProcessInfo {
                pid: pid.as_u32(),
                name: display_name,
                cmd: process
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
                parent_pid: process.parent().map(|p| p.as_u32()),
                start_time: process.start_time(),
                user_id: process.user_id().map(|u| u.to_string().parse().unwrap_or(0)),
                status: format!("{:?}", process.status()),
                exe_path: process.exe().map(|p| p.to_string_lossy().to_string()),
            };

            // PID focus filter (exact match)
            if let Some(target) = self.args.pid {
                if process_info.pid != target {
                    continue;
                }
            }

            // Name filter (substring match)
            if let Some(filter) = &self.args.filter {
                if !process_info.name.to_lowercase().contains(&filter.to_lowercase()) {
                    continue;
                }
            }

            processes.push(process_info);
        }

        let network_connections = if need_network {
            crate::network::get_connections(&pid_names)
        } else {
            vec![]
        };

        let load_avg = System::load_average();

        MonitorSnapshot {
            timestamp: Local::now(),
            processes,
            network_connections,
            system_info: SystemSnapshot {
                total_memory: self.system.total_memory(),
                used_memory: self.system.used_memory(),
                cpu_count: self.system.cpus().len(),
                load_average: load_avg.one,
                uptime: System::uptime(),
                cpu_per_core: self.system.cpus().iter().map(|c| c.cpu_usage()).collect(),
            },
        }
    }

    // ── Display ───────────────────────────────────────────────────────────────────────────────

    fn display_header(&self, snapshot: &MonitorSnapshot) {
        let monitor_uptime = self.start_time.elapsed().as_secs();
        let hidden_count = self.stealth_manager.get_hidden_processes().len()
            + self.stealth_manager.get_hidden_pids().len();
        let renamed_count = self.stealth_manager.get_rename_mappings().len();

        let load_str = if cfg!(windows) {
            "N/A".to_string()
        } else {
            format!("{:.2}", snapshot.system_info.load_average)
        };

        pln!(
            "RProcMon [{}]  Hidden: {} | Renamed: {} | Load: {}",
            snapshot.timestamp.format("%Y-%m-%d %H:%M:%S"),
            hidden_count,
            renamed_count,
            load_str,
        );
        pln!(
            "Memory: {:.1}% ({}/{} MB) | CPUs: {} | Uptime: {}s | Monitor: {}s | Procs: {}",
            (snapshot.system_info.used_memory as f64 / snapshot.system_info.total_memory as f64)
                * 100.0,
            snapshot.system_info.used_memory / 1_048_576,
            snapshot.system_info.total_memory / 1_048_576,
            snapshot.system_info.cpu_count,
            snapshot.system_info.uptime,
            monitor_uptime,
            snapshot.processes.len()
        );
        pln!("Keys: q=quit  s=save  k=SIGTERM  K=SIGKILL  UP/DOWN=select  h=stealth");
    }

    fn display_cpu_cores(&self, snapshot: &MonitorSnapshot) {
        let cores = &snapshot.system_info.cpu_per_core;
        if cores.is_empty() {
            return;
        }

        let per_row: usize = 8;
        let _ = write!(stdout(), "Cores:     ");

        for (i, &usage) in cores.iter().enumerate() {
            if i > 0 && i % per_row == 0 {
                let _ = write!(stdout(), "\r\n           ");
            }
            if usage > 80.0 {
                execute!(stdout(), SetForegroundColor(Color::Red)).unwrap();
            } else if usage > 50.0 {
                execute!(stdout(), SetForegroundColor(Color::Yellow)).unwrap();
            }
            let _ = write!(stdout(), "{:>2}:{:5.1}%  ", i, usage);
            execute!(stdout(), ResetColor).unwrap();
        }
        let _ = write!(stdout(), "\r\n");
        pln!();
    }

    fn display_process_table(&mut self, snapshot: &MonitorSnapshot) {
        pln!("{}", "-".repeat(103));
        pln!(
            "  {:<7} {:<25} {:<8} {:<12} {:<8} {:<8} {:<20}",
            "PID", "NAME", "CPU%", "MEM(KB)", "PPID", "UID", "STATUS"
        );
        pln!("{}", "-".repeat(103));

        let mut sorted = snapshot.processes.clone();
        sorted.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(MAX_DISPLAY);

        if !sorted.is_empty() && self.selected_index >= sorted.len() {
            self.selected_index = sorted.len() - 1;
        }
        self.display_list = sorted.clone();

        let total_mem = snapshot.system_info.total_memory as f64;

        for (i, process) in sorted.iter().enumerate() {
            let is_selected = i == self.selected_index;
            let mem_pct = process.memory as f64 / total_mem * 100.0;

            if is_selected {
                execute!(
                    stdout(),
                    SetForegroundColor(Color::Black),
                    SetBackgroundColor(Color::Cyan)
                )
                .unwrap();
            } else if process.cpu_usage > 50.0 {
                execute!(stdout(), SetForegroundColor(Color::Red)).unwrap();
            } else if process.cpu_usage > 25.0 {
                execute!(stdout(), SetForegroundColor(Color::Yellow)).unwrap();
            } else if mem_pct > 10.0 {
                execute!(stdout(), SetForegroundColor(Color::Magenta)).unwrap();
            } else if mem_pct > 5.0 {
                execute!(stdout(), SetForegroundColor(Color::Blue)).unwrap();
            }

            pln!(
                "{} {:<7} {:<25} {:<8.1} {:<12} {:<8} {:<8} {:<20}",
                if is_selected { ">" } else { " " },
                process.pid,
                truncate_string(&process.name, 25),
                process.cpu_usage,
                process.memory / 1024,
                process.parent_pid.map_or("-".to_string(), |p| p.to_string()),
                process.user_id.map_or("-".to_string(), |u| u.to_string()),
                truncate_string(&process.status, 20)
            );

            execute!(stdout(), ResetColor).unwrap();

            if self.args.verbose && !process.cmd.is_empty() {
                pln!("    CMD: {}", process.cmd.join(" "));
                if let Some(exe) = &process.exe_path {
                    pln!("    EXE: {}", exe);
                }
            }
        }
        pln!();
    }

    fn display_pid_focus(&mut self, snapshot: &MonitorSnapshot) {
        pln!("{}", "=".repeat(103));

        if snapshot.processes.is_empty() {
            pln!("  [PID {} not found or not running]", self.args.pid.unwrap_or(0));
            self.display_list.clear();
            return;
        }

        let proc = &snapshot.processes[0];
        self.display_list = snapshot.processes.clone();
        self.selected_index = 0;

        let mem_pct =
            proc.memory as f64 / snapshot.system_info.total_memory as f64 * 100.0;

        pln!("  PID Focus: {} ({})", proc.pid, proc.name);
        pln!("{}", "-".repeat(103));
        pln!("  Status   : {}", proc.status);
        pln!("  CPU      : {:.2}%", proc.cpu_usage);
        pln!("  Memory   : {} KB  ({:.2}% of total RAM)", proc.memory / 1024, mem_pct);
        pln!(
            "  Parent   : {}",
            proc.parent_pid.map_or("-".to_string(), |p| p.to_string())
        );
        pln!(
            "  UID      : {}",
            proc.user_id.map_or("-".to_string(), |u| u.to_string())
        );

        if let Some(exe) = &proc.exe_path {
            pln!("  Exe      : {}", exe);
        }
        if !proc.cmd.is_empty() {
            pln!("  Command  : {}", proc.cmd.join(" "));
        }

        // Linux-specific detail (working directory, open file descriptors, thread count)
        #[cfg(target_os = "linux")]
        {
            if let Ok(cwd) = std::fs::read_link(format!("/proc/{}/cwd", proc.pid)) {
                pln!("  CWD      : {}", cwd.display());
            }
            if let Ok(fds) = std::fs::read_dir(format!("/proc/{}/fd", proc.pid)) {
                pln!("  Open FDs : {}", fds.count());
            }
            if let Ok(status) =
                std::fs::read_to_string(format!("/proc/{}/status", proc.pid))
            {
                if let Some(line) = status.lines().find(|l| l.starts_with("Threads:")) {
                    if let Some(count) = line.split_whitespace().nth(1) {
                        pln!("  Threads  : {}", count);
                    }
                }
            }
        }

        pln!();

        // Network connections for this PID
        let pid_conns: Vec<&NetworkConnection> = snapshot
            .network_connections
            .iter()
            .filter(|c| c.pid == proc.pid)
            .collect();

        if pid_conns.is_empty() {
            pln!("  No network connections.");
        } else {
            pln!(
                "  {:<5} {:<24} {:<24} {:<12}",
                "PROTO", "LOCAL", "REMOTE", "STATE"
            );
            pln!("  {}", "-".repeat(70));
            for conn in &pid_conns {
                if conn.state == "ESTABLISHED" || conn.state == "ESTABLISHED" {
                    execute!(stdout(), SetForegroundColor(Color::Green)).unwrap();
                } else if conn.state == "LISTEN" || conn.state == "LISTENING" {
                    execute!(stdout(), SetForegroundColor(Color::Cyan)).unwrap();
                }
                pln!(
                    "  {:<5} {:<24} {:<24} {:<12}",
                    conn.protocol,
                    conn.local_addr,
                    conn.remote_addr,
                    conn.state,
                );
                execute!(stdout(), ResetColor).unwrap();
            }
        }

        pln!("{}", "=".repeat(103));
        pln!("Keys: q=quit  s=save  k=SIGTERM (this process)  K=SIGKILL (this process)");
    }

    fn display_network_connections(&self, snapshot: &MonitorSnapshot) {
        if snapshot.network_connections.is_empty() {
            return;
        }

        let mut sorted: Vec<&NetworkConnection> = snapshot
            .network_connections
            .iter()
            .filter(|c| c.state != "CLOSE" && c.state != "UNKNOWN")
            .collect();

        sorted.sort_by_key(|c| match c.state.as_str() {
            "ESTABLISHED" => 0u8,
            "LISTEN" | "LISTENING" => 1,
            _ => 2,
        });

        pln!(
            "Network [{} sockets, showing {}]",
            snapshot.network_connections.len(),
            sorted.len().min(15)
        );
        pln!("{}", "-".repeat(103));
        pln!(
            "{:<5} {:<20} {:<24} {:<24} {:<12}",
            "PROTO", "PROCESS(PID)", "LOCAL", "REMOTE", "STATE"
        );
        pln!("{}", "-".repeat(103));

        for conn in sorted.iter().take(15) {
            match conn.state.as_str() {
                "ESTABLISHED" => {
                    execute!(stdout(), SetForegroundColor(Color::Green)).unwrap();
                }
                "LISTEN" | "LISTENING" => {
                    execute!(stdout(), SetForegroundColor(Color::Cyan)).unwrap();
                }
                _ => {}
            }

            let proc_label = if conn.pid > 0 {
                format!("{}({})", truncate_string(&conn.process_name, 12), conn.pid)
            } else {
                "kernel".to_string()
            };

            pln!(
                "{:<5} {:<20} {:<24} {:<24} {:<12}",
                conn.protocol,
                truncate_string(&proc_label, 20),
                conn.local_addr,
                conn.remote_addr,
                conn.state,
            );

            execute!(stdout(), ResetColor).unwrap();
        }
        pln!();
    }

    // ── Kill ──────────────────────────────────────────────────────────────────────────────────

    fn kill_selected(&mut self, force: bool) {
        let Some(info) = self.display_list.get(self.selected_index) else {
            return;
        };
        let pid = info.pid;
        let name = info.name.clone();

        let pid_sysinfo = sysinfo::Pid::from(pid as usize);
        let sent = if let Some(proc) = self.system.process(pid_sysinfo) {
            if force {
                proc.kill()
            } else {
                proc.kill_with(Signal::Term).unwrap_or(false)
            }
        } else {
            false
        };

        self.kill_status = Some(if sent {
            format!(
                "Sent {} to {} (PID: {})",
                if force { "SIGKILL" } else { "SIGTERM" },
                name,
                pid
            )
        } else {
            format!("Failed to signal {} (PID: {})", name, pid)
        });
    }

    // ── Alerting ──────────────────────────────────────────────────────────────────────────────

    fn check_for_new_processes(&mut self, snapshot: &MonitorSnapshot) {
        if self.args.alert {
            let current: std::collections::HashSet<u32> =
                snapshot.processes.iter().map(|p| p.pid).collect();
            let previous: std::collections::HashSet<u32> =
                self.previous_processes.keys().cloned().collect();

            let new_pids: Vec<u32> = current.difference(&previous).cloned().collect();

            if !new_pids.is_empty() && !self.previous_processes.is_empty() {
                execute!(stdout(), SetForegroundColor(Color::Green)).unwrap();
                pln!("*** NEW PROCESSES:");
                for pid in new_pids {
                    if let Some(p) = snapshot.processes.iter().find(|p| p.pid == pid) {
                        pln!(
                            "  [{}] {} (PID: {})",
                            snapshot.timestamp.format("%H:%M:%S"),
                            p.name,
                            p.pid
                        );
                        if self.args.verbose {
                            pln!("    CMD: {}", p.cmd.join(" "));
                            if let Some(exe) = &p.exe_path {
                                pln!("    EXE: {}", exe);
                            }
                        }
                    }
                }
                execute!(stdout(), ResetColor).unwrap();
                pln!();
            }
        }

        self.previous_processes.clear();
        for process in &snapshot.processes {
            self.previous_processes.insert(process.pid, process.clone());
        }
    }

    // ── Export ────────────────────────────────────────────────────────────────────────────────

    fn save_snapshot(&self) -> io::Result<()> {
        let Some(latest) = self.snapshots.last() else {
            return Ok(());
        };
        let ts = latest.timestamp.format("%Y%m%d_%H%M%S");
        match self.args.format {
            OutputFormat::Json => {
                let filename = format!("rprocmon_snapshot_{}.json", ts);
                std::fs::write(&filename, serde_json::to_string_pretty(latest)?)?;
                pln!("Snapshot saved: {}", filename);
            }
            OutputFormat::Csv => {
                let filename = format!("rprocmon_snapshot_{}.csv", ts);
                std::fs::write(&filename, self.snapshot_to_csv(latest))?;
                pln!("Snapshot saved: {}", filename);
            }
        }
        let _ = stdout().flush();
        Ok(())
    }

    fn write_output(&self, path: &str) -> io::Result<()> {
        match self.args.format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&self.snapshots)?;
                std::fs::write(path, json)?;
            }
            OutputFormat::Csv => {
                let mut rows = vec![
                    "timestamp,pid,name,cpu_pct,memory_kb,ppid,uid,status,exe_path,cmd"
                        .to_string(),
                ];
                for snap in &self.snapshots {
                    for row in self.snapshot_to_csv_rows(snap) {
                        rows.push(row);
                    }
                }
                std::fs::write(path, rows.join("\n"))?;
            }
        }
        pln!("Output saved: {}", path);
        let _ = stdout().flush();
        Ok(())
    }

    fn snapshot_to_csv(&self, snap: &MonitorSnapshot) -> String {
        let mut rows = vec![
            "timestamp,pid,name,cpu_pct,memory_kb,ppid,uid,status,exe_path,cmd".to_string(),
        ];
        for row in self.snapshot_to_csv_rows(snap) {
            rows.push(row);
        }
        rows.join("\n")
    }

    fn snapshot_to_csv_rows(&self, snap: &MonitorSnapshot) -> Vec<String> {
        let ts = snap.timestamp.format("%Y-%m-%dT%H:%M:%S").to_string();
        snap.processes
            .iter()
            .map(|p| {
                format!(
                    "{},{},{},{:.2},{},{},{},{},{},\"{}\"",
                    ts,
                    p.pid,
                    p.name,
                    p.cpu_usage,
                    p.memory / 1024,
                    p.parent_pid.map_or(String::new(), |x| x.to_string()),
                    p.user_id.map_or(String::new(), |x| x.to_string()),
                    p.status,
                    p.exe_path.as_deref().unwrap_or(""),
                    p.cmd.join(" ").replace('"', "\"\""),
                )
            })
            .collect()
    }
}
