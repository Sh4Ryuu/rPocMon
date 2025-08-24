use sysinfo::{System, Networks};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::thread;
use chrono::Local;
use crossterm::{
    execute,
    terminal::{Clear, ClearType},
    cursor::{MoveTo, Hide, Show},
    style::{Color, SetForegroundColor, ResetColor},
    event::{self, Event, KeyCode},
};
use std::io::{self, stdout};

use crate::args::Args;
use crate::types::{ProcessInfo, NetworkConnection, MonitorSnapshot, SystemSnapshot};
use crate::utils::truncate_string;
use crate::stealth::StealthManager;

pub struct ProcessMonitor {
    system: System,
    previous_processes: HashMap<u32, ProcessInfo>,
    args: Args,
    start_time: Instant,
    snapshots: Vec<MonitorSnapshot>,
    stealth_manager: StealthManager,
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
        }
    }

    pub fn run(&mut self) -> io::Result<()> {
        println!("🔍 RProcMon - Rust Process Monitor (Stealth Mode Active)");
        println!("Press 'q' to quit, 's' to save snapshot, 'h' to toggle stealth config\n");

        // Hide cursor for cleaner output
        execute!(stdout(), Hide)?;

        loop {
            self.system.refresh_all();

            let snapshot = self.collect_snapshot();
            self.display_processes(&snapshot);

            if self.args.network {
                self.display_network_connections(&snapshot);
            }

            self.check_for_new_processes(&snapshot);
            self.snapshots.push(snapshot);

            // Check for user input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('s') => self.save_current_snapshot()?,
                        KeyCode::Char('c') => {
                            execute!(stdout(), Clear(ClearType::All))?;
                        }
                        KeyCode::Char('h') => {
                            execute!(stdout(), Show)?;
                            if let Err(e) = self.stealth_manager.interactive_config() {
                                println!("Error configuring stealth: {}", e);
                            }
                            execute!(stdout(), Hide)?;
                        }
                        _ => {}
                    }
                }
            }

            thread::sleep(Duration::from_secs(self.args.interval));

            // Clear screen for next update
            execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
        }

        execute!(stdout(), Show)?;

        if let Some(output_path) = &self.args.output {
            self.save_all_snapshots(output_path)?;
        }

        Ok(())
    }

    fn collect_snapshot(&mut self) -> MonitorSnapshot {
        let mut processes = Vec::new();
        let mut network_connections = Vec::new();

        // Collect process information with stealth filtering
        for (pid, process) in self.system.processes() {
            let original_name = process.name().to_string_lossy().to_string();

            // Apply stealth filtering - skip hidden processes
            if self.stealth_manager.is_process_hidden(&original_name) ||
                self.stealth_manager.is_pid_hidden(pid.as_u32()) {
                continue;
            }

            // Get display name (potentially renamed)
            let display_name = self.stealth_manager.get_display_name(&original_name);

            let process_info = ProcessInfo {
                pid: pid.as_u32(),
                name: display_name, // Use display name instead of original
                cmd: process.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
                parent_pid: process.parent().map(|p| p.as_u32()),
                start_time: process.start_time(),
                user_id: process.user_id().map(|u| u.to_string().parse().unwrap_or(0)),
                status: format!("{:?}", process.status()),
                exe_path: process.exe().map(|p| p.to_string_lossy().to_string()),
            };

            // Apply original filter if specified (but not if it's stealth-config)
            if let Some(filter) = &self.args.filter {
                if filter != "stealth-config" &&
                    !process_info.name.to_lowercase().contains(&filter.to_lowercase()) {
                    continue;
                }
            }

            processes.push(process_info);
        }

        // Collect network information if requested
        if self.args.network {
            let networks = Networks::new_with_refreshed_list();
            for (interface_name, network) in &networks {
                if network.received() > 0 || network.transmitted() > 0 {
                    let conn = NetworkConnection {
                        process_name: format!("Interface: {}", interface_name),
                        pid: 0,
                        local_addr: "0.0.0.0".to_string(),
                        remote_addr: "0.0.0.0".to_string(),
                        state: "ACTIVE".to_string(),
                        protocol: "TCP/UDP".to_string(),
                    };
                    network_connections.push(conn);
                }
            }
        }

        let system_info = SystemSnapshot {
            total_memory: self.system.total_memory(),
            used_memory: self.system.used_memory(),
            cpu_count: self.system.cpus().len(),
            load_average: 0.0,
            uptime: System::uptime(),
        };

        MonitorSnapshot {
            timestamp: Local::now(),
            processes,
            network_connections,
            system_info,
        }
    }

    fn display_processes(&self, snapshot: &MonitorSnapshot) {
        let monitor_uptime = self.start_time.elapsed().as_secs();

        // Show stealth status
        let hidden_count = self.stealth_manager.get_hidden_processes().len() +
            self.stealth_manager.get_hidden_pids().len();
        let renamed_count = self.stealth_manager.get_rename_mappings().len();

        println!("📊 System Overview [{}] 🥷 Hidden: {} | Renamed: {}",
                 snapshot.timestamp.format("%Y-%m-%d %H:%M:%S"),
                 hidden_count,
                 renamed_count);

        println!("Memory: {:.1}% ({}/{} MB) | CPUs: {} | Uptime: {}s | Monitor: {}s | Processes: {}",
                 (snapshot.system_info.used_memory as f64 / snapshot.system_info.total_memory as f64) * 100.0,
                 snapshot.system_info.used_memory / 1_048_576,
                 snapshot.system_info.total_memory / 1_048_576,
                 snapshot.system_info.cpu_count,
                 snapshot.system_info.uptime,
                 monitor_uptime,
                 snapshot.processes.len()
        );
        println!("{}", "─".repeat(120));

        // Header
        println!("{:<8} {:<25} {:<8} {:<12} {:<8} {:<10} {:<20}",
                 "PID", "NAME", "CPU%", "MEMORY(KB)", "PPID", "USER_ID", "STATUS");
        println!("{}", "─".repeat(120));

        // Sort processes by CPU usage
        let mut sorted_processes = snapshot.processes.clone();
        sorted_processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal));

        // Display top processes
        for (_, process) in sorted_processes.iter().take(20).enumerate() {
            // Highlight high CPU usage
            if process.cpu_usage > 50.0 {
                execute!(stdout(), SetForegroundColor(Color::Red)).unwrap();
            } else if process.cpu_usage > 25.0 {
                execute!(stdout(), SetForegroundColor(Color::Yellow)).unwrap();
            }

            println!("{:<8} {:<25} {:<8.1} {:<12} {:<8} {:<10} {:<20}",
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
                println!("    CMD: {}", process.cmd.join(" "));
                if let Some(exe_path) = &process.exe_path {
                    println!("    EXE: {}", exe_path);
                }
            }
        }

        println!();
    }

    fn display_network_connections(&self, snapshot: &MonitorSnapshot) {
        if !snapshot.network_connections.is_empty() {
            println!("🌐 Network Activity");
            println!("{}", "─".repeat(80));

            for conn in &snapshot.network_connections {
                println!("{:<20} {:<8} {:<20} -> {:<20} [{}]",
                         truncate_string(&conn.process_name, 20),
                         conn.pid,
                         conn.local_addr,
                         conn.remote_addr,
                         conn.state
                );
            }
            println!();
        }
    }

    fn check_for_new_processes(&mut self, snapshot: &MonitorSnapshot) {
        if self.args.alert {
            let current_pids: std::collections::HashSet<u32> =
                snapshot.processes.iter().map(|p| p.pid).collect();
            let previous_pids: std::collections::HashSet<u32> =
                self.previous_processes.keys().cloned().collect();

            let new_pids: Vec<u32> = current_pids.difference(&previous_pids).cloned().collect();

            if !new_pids.is_empty() && !self.previous_processes.is_empty() {
                execute!(stdout(), SetForegroundColor(Color::Green)).unwrap();
                println!("🚨 NEW PROCESSES DETECTED:");
                for pid in new_pids {
                    if let Some(process) = snapshot.processes.iter().find(|p| p.pid == pid) {
                        println!("  [{}] {} (PID: {})",
                                 snapshot.timestamp.format("%H:%M:%S"),
                                 process.name,
                                 process.pid
                        );
                        if self.args.verbose {
                            println!("    CMD: {}", process.cmd.join(" "));
                            if let Some(exe_path) = &process.exe_path {
                                println!("    EXE: {}", exe_path);
                            }
                        }
                    }
                }
                execute!(stdout(), ResetColor).unwrap();
                println!();
            }
        }

        // Update previous processes
        self.previous_processes.clear();
        for process in &snapshot.processes {
            self.previous_processes.insert(process.pid, process.clone());
        }
    }

    fn save_current_snapshot(&self) -> io::Result<()> {
        if let Some(latest) = self.snapshots.last() {
            let filename = format!("rprocmon_snapshot_{}.json",
                                   latest.timestamp.format("%Y%m%d_%H%M%S"));

            let json = serde_json::to_string_pretty(latest)?;
            std::fs::write(&filename, json)?;

            println!("💾 Snapshot saved to: {}", filename);
        }
        Ok(())
    }

    fn save_all_snapshots(&self, output_path: &str) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.snapshots)?;
        std::fs::write(output_path, json)?;
        println!("💾 All snapshots saved to: {}", output_path);
        Ok(())
    }
}