use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealthConfig {
    // Processes to completely hide by name
    pub hidden_processes: Vec<String>,
    // Process name mappings (original_name -> display_name)
    pub renamed_processes: HashMap<String, String>,
    // Processes to hide by PID
    pub hidden_pids: Vec<u32>,
}

impl Default for StealthConfig {
    fn default() -> Self {
        Self {
            hidden_processes: Vec::new(),
            renamed_processes: HashMap::new(),
            hidden_pids: Vec::new(),
        }
    }
}

pub struct StealthManager {
    config: StealthConfig,
    config_path: String,
}

impl StealthManager {
    pub fn new() -> Self {
        let config_path = "stealth_config.json".to_string();
        let config = Self::load_config(&config_path).unwrap_or_default();

        Self {
            config,
            config_path,
        }
    }

    /// Load stealth configuration from file
    fn load_config(path: &str) -> Result<StealthConfig, Box<dyn std::error::Error>> {
        if !Path::new(path).exists() {
            // Create default config file
            let default_config = StealthConfig::default();
            let json = serde_json::to_string_pretty(&default_config)?;
            fs::write(path, json)?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(path)?;
        let config: StealthConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save current configuration to file
    pub fn save_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, json)?;
        Ok(())
    }

    /// Add a process name to the hidden list
    pub fn hide_process(&mut self, process_name: &str) {
        if !self.config.hidden_processes.contains(&process_name.to_string()) {
            self.config.hidden_processes.push(process_name.to_string());
        }
    }

    /// Remove a process name from the hidden list
    pub fn unhide_process(&mut self, process_name: &str) {
        self.config.hidden_processes.retain(|p| p != process_name);
    }

    /// Add a PID to the hidden list
    pub fn hide_pid(&mut self, pid: u32) {
        if !self.config.hidden_pids.contains(&pid) {
            self.config.hidden_pids.push(pid);
        }
    }

    /// Remove a PID from the hidden list
    pub fn unhide_pid(&mut self, pid: u32) {
        self.config.hidden_pids.retain(|&p| p != pid);
    }

    /// Add or update a process name mapping
    pub fn rename_process(&mut self, original_name: &str, display_name: &str) {
        self.config.renamed_processes.insert(
            original_name.to_string(),
            display_name.to_string(),
        );
    }

    /// Remove a process name mapping
    pub fn remove_rename(&mut self, original_name: &str) {
        self.config.renamed_processes.remove(original_name);
    }

    /// Check if a process should be hidden by name
    pub fn is_process_hidden(&self, process_name: &str) -> bool {
        self.config.hidden_processes.iter()
            .any(|hidden| process_name.to_lowercase().contains(&hidden.to_lowercase()))
    }

    /// Check if a process should be hidden by PID
    pub fn is_pid_hidden(&self, pid: u32) -> bool {
        self.config.hidden_pids.contains(&pid)
    }

    /// Get the display name for a process (renamed if configured)
    pub fn get_display_name(&self, original_name: &str) -> String {
        // Check for exact match first
        if let Some(renamed) = self.config.renamed_processes.get(original_name) {
            return renamed.clone();
        }

        // Check for partial matches
        for (original, renamed) in &self.config.renamed_processes {
            if original_name.to_lowercase().contains(&original.to_lowercase()) {
                return renamed.clone();
            }
        }

        original_name.to_string()
    }

    /// Get list of hidden processes
    pub fn get_hidden_processes(&self) -> &Vec<String> {
        &self.config.hidden_processes
    }

    /// Get list of hidden PIDs
    pub fn get_hidden_pids(&self) -> &Vec<u32> {
        &self.config.hidden_pids
    }

    /// Get process rename mappings
    pub fn get_rename_mappings(&self) -> &HashMap<String, String> {
        &self.config.renamed_processes
    }

    /// Clear all stealth configurations
    pub fn clear_all(&mut self) {
        self.config.hidden_processes.clear();
        self.config.hidden_pids.clear();
        self.config.renamed_processes.clear();
    }

    /// Interactive configuration menu
    pub fn interactive_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::{self, Write};

        loop {
            println!("\n🥷 Stealth Configuration Menu");
            println!("1. Hide process by name");
            println!("2. Hide process by PID");
            println!("3. Rename process");
            println!("4. Remove hidden process");
            println!("5. Remove hidden PID");
            println!("6. Remove process rename");
            println!("7. Show current config");
            println!("8. Clear all config");
            println!("9. Save and exit");
            print!("Select option (1-9): ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let choice = input.trim();

            match choice {
                "1" => {
                    print!("Enter process name to hide: ");
                    io::stdout().flush()?;
                    let mut name = String::new();
                    io::stdin().read_line(&mut name)?;
                    self.hide_process(name.trim());
                    println!("✅ Process '{}' added to hidden list", name.trim());
                }
                "2" => {
                    print!("Enter PID to hide: ");
                    io::stdout().flush()?;
                    let mut pid_input = String::new();
                    io::stdin().read_line(&mut pid_input)?;
                    if let Ok(pid) = pid_input.trim().parse::<u32>() {
                        self.hide_pid(pid);
                        println!("✅ PID {} added to hidden list", pid);
                    } else {
                        println!("❌ Invalid PID format");
                    }
                }
                "3" => {
                    print!("Enter original process name: ");
                    io::stdout().flush()?;
                    let mut original = String::new();
                    io::stdin().read_line(&mut original)?;

                    print!("Enter display name: ");
                    io::stdout().flush()?;
                    let mut display = String::new();
                    io::stdin().read_line(&mut display)?;

                    self.rename_process(original.trim(), display.trim());
                    println!("✅ Process '{}' will display as '{}'", original.trim(), display.trim());
                }
                "4" => {
                    print!("Enter process name to unhide: ");
                    io::stdout().flush()?;
                    let mut name = String::new();
                    io::stdin().read_line(&mut name)?;
                    self.unhide_process(name.trim());
                    println!("✅ Process '{}' removed from hidden list", name.trim());
                }
                "5" => {
                    print!("Enter PID to unhide: ");
                    io::stdout().flush()?;
                    let mut pid_input = String::new();
                    io::stdin().read_line(&mut pid_input)?;
                    if let Ok(pid) = pid_input.trim().parse::<u32>() {
                        self.unhide_pid(pid);
                        println!("✅ PID {} removed from hidden list", pid);
                    } else {
                        println!("❌ Invalid PID format");
                    }
                }
                "6" => {
                    print!("Enter process name to remove rename: ");
                    io::stdout().flush()?;
                    let mut name = String::new();
                    io::stdin().read_line(&mut name)?;
                    self.remove_rename(name.trim());
                    println!("✅ Rename mapping for '{}' removed", name.trim());
                }
                "7" => {
                    self.display_current_config();
                }
                "8" => {
                    self.clear_all();
                    println!("✅ All stealth configurations cleared");
                }
                "9" => {
                    self.save_config()?;
                    println!("✅ Configuration saved!");
                    break;
                }
                _ => {
                    println!("❌ Invalid option");
                }
            }
        }

        Ok(())
    }

    /// Display current stealth configuration
    fn display_current_config(&self) {
        println!("\n📋 Current Stealth Configuration:");

        if !self.config.hidden_processes.is_empty() {
            println!("🙈 Hidden Processes:");
            for process in &self.config.hidden_processes {
                println!("  - {}", process);
            }
        }

        if !self.config.hidden_pids.is_empty() {
            println!("🔢 Hidden PIDs:");
            for pid in &self.config.hidden_pids {
                println!("  - {}", pid);
            }
        }

        if !self.config.renamed_processes.is_empty() {
            println!("🎭 Process Renames:");
            for (original, display) in &self.config.renamed_processes {
                println!("  - {} -> {}", original, display);
            }
        }

        if self.config.hidden_processes.is_empty()
            && self.config.hidden_pids.is_empty()
            && self.config.renamed_processes.is_empty() {
            println!("  No stealth configurations active");
        }
    }
}