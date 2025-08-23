use serde::{Deserialize, Serialize};
use chrono::{DateTime, Local};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmd: Vec<String>,
    pub cpu_usage: f32,
    pub memory: u64,
    pub parent_pid: Option<u32>,
    pub start_time: u64,
    pub user_id: Option<u32>,
    pub status: String,
    pub exe_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConnection {
    pub process_name: String,
    pub pid: u32,
    pub local_addr: String,
    pub remote_addr: String,
    pub state: String,
    pub protocol: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorSnapshot {
    pub timestamp: DateTime<Local>,
    pub processes: Vec<ProcessInfo>,
    pub network_connections: Vec<NetworkConnection>,
    pub system_info: SystemSnapshot,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub total_memory: u64,
    pub used_memory: u64,
    pub cpu_count: usize,
    pub load_average: f64,
    pub uptime: u64,
}