use crate::types::NetworkConnection;

pub fn get_connections(pids: &[(u32, String)]) -> Vec<NetworkConnection> {
    #[cfg(any(target_os = "linux", windows))]
    return platform::get_connections(pids);

    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = pids;
        vec![]
    }
}

// ── Linux: parse /proc/net/tcp and /proc/net/udp, resolve inodes via /proc/<pid>/fd ──────────

#[cfg(target_os = "linux")]
mod platform {
    use std::collections::HashMap;
    use std::fs;
    use std::net::Ipv4Addr;
    use crate::types::NetworkConnection;

    const TCP_STATES: &[&str] = &[
        "UNKNOWN", "ESTABLISHED", "SYN_SENT", "SYN_RECV",
        "FIN_WAIT1", "FIN_WAIT2", "TIME_WAIT", "CLOSE",
        "CLOSE_WAIT", "LAST_ACK", "LISTEN", "CLOSING",
    ];

    fn state_name(hex: &str) -> &'static str {
        let idx = usize::from_str_radix(hex, 16).unwrap_or(0);
        TCP_STATES.get(idx).copied().unwrap_or("UNKNOWN")
    }

    fn parse_hex_addr(s: &str) -> Option<(String, u16)> {
        let (ip_hex, port_hex) = s.split_once(':')?;
        let raw = u32::from_str_radix(ip_hex, 16).ok()?;
        let port = u16::from_str_radix(port_hex, 16).ok()?;
        // /proc/net stores IPv4 as little-endian u32 on x86; swap to network byte order
        Some((Ipv4Addr::from(raw.swap_bytes()).to_string(), port))
    }

    fn parse_proc_net(path: &str) -> Vec<(String, String, String, u64)> {
        fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .skip(1)
            .filter_map(|line| {
                let f: Vec<&str> = line.split_whitespace().collect();
                if f.len() < 10 {
                    return None;
                }
                let (lip, lp) = parse_hex_addr(f[1])?;
                let (rip, rp) = parse_hex_addr(f[2])?;
                let inode: u64 = f[9].parse().ok()?;
                Some((
                    format!("{}:{}", lip, lp),
                    format!("{}:{}", rip, rp),
                    state_name(f[3]).to_string(),
                    inode,
                ))
            })
            .collect()
    }

    fn build_inode_pid_map(pids: &[(u32, String)]) -> HashMap<u64, (u32, String)> {
        let mut map = HashMap::new();
        for (pid, name) in pids {
            let Ok(entries) = fs::read_dir(format!("/proc/{}/fd", pid)) else {
                continue;
            };
            for entry in entries.flatten() {
                let Ok(link) = fs::read_link(entry.path()) else {
                    continue;
                };
                let s = link.to_string_lossy();
                if let Some(inner) = s
                    .strip_prefix("socket:[")
                    .and_then(|x| x.strip_suffix(']'))
                {
                    if let Ok(inode) = inner.parse::<u64>() {
                        map.insert(inode, (*pid, name.clone()));
                    }
                }
            }
        }
        map
    }

    pub fn get_connections(pids: &[(u32, String)]) -> Vec<NetworkConnection> {
        let inode_map = build_inode_pid_map(pids);
        let mut conns = Vec::new();
        for (path, proto) in &[("/proc/net/tcp", "TCP"), ("/proc/net/udp", "UDP")] {
            for (local, remote, state, inode) in parse_proc_net(path) {
                let (pid, name) = inode_map.get(&inode).cloned().unwrap_or((0, String::new()));
                conns.push(NetworkConnection {
                    process_name: name,
                    pid,
                    local_addr: local,
                    remote_addr: remote,
                    state,
                    protocol: proto.to_string(),
                });
            }
        }
        conns
    }
}

// ── Windows: parse `netstat -ano` output, look up process names from sysinfo pid list ─────────

#[cfg(windows)]
mod platform {
    use std::collections::HashMap;
    use std::process::Command;
    use crate::types::NetworkConnection;

    pub fn get_connections(pids: &[(u32, String)]) -> Vec<NetworkConnection> {
        let pid_map: HashMap<u32, String> = pids.iter().cloned().collect();

        let output = match Command::new("netstat").args(["-ano"]).output() {
            Ok(o) => o,
            Err(_) => return vec![],
        };

        let text = String::from_utf8_lossy(&output.stdout);
        let mut conns = Vec::new();

        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            // TCP  <local>  <remote>  <state>  <pid>
            if parts[0] == "TCP" && parts.len() >= 5 {
                let pid: u32 = parts[4].parse().unwrap_or(0);
                conns.push(NetworkConnection {
                    process_name: pid_map.get(&pid).cloned().unwrap_or_default(),
                    pid,
                    local_addr: parts[1].to_string(),
                    remote_addr: parts[2].to_string(),
                    state: parts[3].to_string(),
                    protocol: "TCP".to_string(),
                });
            }

            // UDP  <local>  <remote>  <pid>   (no state column)
            if parts[0] == "UDP" && parts.len() >= 4 {
                let pid: u32 = parts[3].parse().unwrap_or(0);
                conns.push(NetworkConnection {
                    process_name: pid_map.get(&pid).cloned().unwrap_or_default(),
                    pid,
                    local_addr: parts[1].to_string(),
                    remote_addr: parts[2].to_string(),
                    state: String::new(),
                    protocol: "UDP".to_string(),
                });
            }
        }

        conns
    }
}
