use std::process::Command;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct BootTimeEntry {
    pub kernel_version: String,
    pub boot_time: f64, // in seconds
    pub timestamp: String,
}

pub fn get_boot_times() -> Vec<BootTimeEntry> {
    let mut entries = Vec::new();
    
    // Get current boot time from systemd-analyze time
    if let Ok(time_output) = Command::new("systemd-analyze")
        .arg("time")
        .output()
    {
        let time_stdout = String::from_utf8_lossy(&time_output.stdout);
        // Parse format like "Startup finished in 2.345s (kernel) + 1.234s (initrd) + 5.678s (userspace) = 9.257s"
        // or simpler format like "Startup finished in 9.257s"
        let time_re = Regex::new(r"Startup finished in (?:[^=]+= )?(\d+\.?\d*)\s*(s|ms)").ok();
        
        if let Some(caps) = time_re.as_ref().and_then(|re| re.captures(&time_stdout)) {
            if let (Some(time_str), Some(unit)) = (caps.get(1), caps.get(2)) {
                if let Ok(time_val) = time_str.as_str().parse::<f64>() {
                    let boot_time = if unit.as_str() == "ms" {
                        time_val / 1000.0
                    } else {
                        time_val
                    };
                    
                    // Get kernel version
                    let kernel_version = get_current_kernel().unwrap_or_else(|| "Unknown".to_string());
                    
                    entries.push(BootTimeEntry {
                        kernel_version,
                        boot_time,
                        timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    });
                }
            }
        }
    }
    
    // Try to get historical boot times from journalctl
    // Use systemd-analyze time -b to get boot time for specific boots
    if let Ok(output) = Command::new("journalctl")
        .arg("--list-boots")
        .arg("--no-pager")
        .arg("-n")
        .arg("10")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse boot list output
        // Format: 0 abc123... 2024-01-01 12:00:00 +0800 2024-01-01 12:05:00 +0800
        let boot_line_re = Regex::new(r"^\s*(\d+)\s+(\S+)\s+([^\s]+\s+[^\s]+\s+[^\s]+)\s+([^\s]+\s+[^\s]+\s+[^\s]+)").ok();
        
        for line in stdout.lines() {
            if let Some(caps) = boot_line_re.as_ref().and_then(|re| re.captures(line)) {
                if let (Some(boot_idx_str), Some(boot_start)) = (caps.get(1), caps.get(3)) {
                    let boot_start_str = boot_start.as_str();
                    let boot_idx = boot_idx_str.as_str().parse::<i32>().unwrap_or(-1);
                    
                    // Skip current boot (index 0) as we already have it from systemd-analyze time
                    if boot_idx <= 0 {
                        continue;
                    }
                    
                    // Try to get boot time for this specific boot using systemd-analyze time -b
                    let boot_time = get_boot_time_for_boot_index(boot_idx);
                    
                    // Get kernel version from journal for this boot
                    let kernel_version = get_kernel_version_for_boot(boot_idx)
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    // Only add if we got a valid boot time and not already added
                    if boot_time > 0.0 && entries.iter().all(|e| e.timestamp != boot_start_str) {
                        entries.push(BootTimeEntry {
                            kernel_version,
                            boot_time,
                            timestamp: boot_start_str.to_string(),
                        });
                    }
                }
            }
        }
    }
    
    // Sort by timestamp (newest first)
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    entries
}

fn get_current_kernel() -> Option<String> {
    if let Ok(uname) = std::process::Command::new("uname")
        .arg("-r")
        .output()
    {
        if uname.status.success() {
            return String::from_utf8(uname.stdout).ok()
                .map(|s| s.trim().to_string());
        }
    }
    None
}

fn get_boot_time_for_boot_index(boot_idx: i32) -> f64 {
    // Try to get boot time from journalctl for specific boot
    // Look for "Startup finished" message in systemd logs
    if let Ok(output) = Command::new("journalctl")
        .arg("-b")
        .arg(&boot_idx.to_string())
        .arg("--no-pager")
        .arg("--grep")
        .arg("Startup finished")
        .arg("-n")
        .arg("1")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Parse format like "Startup finished in 2.345s (kernel) + 1.234s (initrd) + 5.678s (userspace) = 9.257s"
        // or simpler format like "Startup finished in 9.257s"
        let time_re = Regex::new(r"Startup finished in (?:[^=]+= )?(\d+\.?\d*)\s*(s|ms)").ok();
        
        if let Some(caps) = time_re.as_ref().and_then(|re| re.captures(&stdout)) {
            if let (Some(time_str), Some(unit)) = (caps.get(1), caps.get(2)) {
                if let Ok(time_val) = time_str.as_str().parse::<f64>() {
                    return if unit.as_str() == "ms" {
                        time_val / 1000.0
                    } else {
                        time_val
                    };
                }
            }
        }
    }
    
    // Fallback: try systemd-analyze time with boot ID if available
    // This is less reliable but may work in some cases
    0.0
}

fn get_kernel_version_for_boot(boot_idx: i32) -> Option<String> {
    // Get kernel version from journal for specific boot
    if let Ok(output) = Command::new("journalctl")
        .arg("-b")
        .arg(&boot_idx.to_string())
        .arg("--no-pager")
        .arg("--grep")
        .arg("Linux version")
        .arg("-n")
        .arg("1")
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Parse kernel version from log line like "Linux version 5.15.0-91-generic"
        let kernel_re = Regex::new(r"Linux version (\S+)").ok();
        if let Some(caps) = kernel_re.as_ref().and_then(|re| re.captures(&stdout)) {
            if let Some(version) = caps.get(1) {
                return Some(version.as_str().to_string());
            }
        }
    }
    None
}

// Helper function to get boot time for a specific kernel version (if needed in the future)
#[allow(dead_code)]
pub fn get_boot_time_for_kernel(kernel_version: &str) -> Option<f64> {
    let entries = get_boot_times();
    entries.iter()
        .find(|e| e.kernel_version.contains(kernel_version))
        .map(|e| e.boot_time)
}

pub fn format_boot_time(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{:.2}s", seconds)
    } else {
        let minutes = (seconds / 60.0) as u64;
        let secs = seconds % 60.0;
        format!("{}m {:.2}s", minutes, secs)
    }
}
