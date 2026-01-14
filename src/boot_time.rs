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
    
    // Try to get boot time from systemd-analyze time
    // This gives us the current boot time
    if let Ok(time_output) = Command::new("systemd-analyze")
        .arg("time")
        .output()
    {
        let time_stdout = String::from_utf8_lossy(&time_output.stdout);
        let time_re = Regex::new(r"(\d+\.?\d*)\s*(s|ms)").ok();
        
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
                if let (Some(_boot_id), Some(boot_start), Some(boot_end)) = 
                    (caps.get(1), caps.get(3), caps.get(4)) {
                    
                    let boot_start_str = boot_start.as_str();
                    let boot_end_str = boot_end.as_str();
                    
                    // Try to get kernel version from dmesg or uname
                    let kernel_version = get_current_kernel().unwrap_or_else(|| "Unknown".to_string());
                    
                    // Calculate boot time (difference between boot_end and boot_start)
                    if let (Ok(start_time), Ok(end_time)) = (
                        parse_datetime(boot_start_str),
                        parse_datetime(boot_end_str)
                    ) {
                        let boot_duration = end_time.duration_since(start_time)
                            .unwrap_or_default()
                            .as_secs_f64();
                        
                        // Only add if not already added (avoid duplicates)
                        if entries.iter().all(|e| e.timestamp != boot_start_str) {
                            entries.push(BootTimeEntry {
                                kernel_version: kernel_version.clone(),
                                boot_time: boot_duration,
                                timestamp: boot_start_str.to_string(),
                            });
                        }
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

fn parse_datetime(datetime_str: &str) -> Result<std::time::SystemTime, String> {
    // Parse format: "2024-01-01 12:00:00 +0800"
    use chrono::DateTime;
    
    // Try to parse with timezone
    if let Ok(dt) = DateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S %z") {
        return Ok(dt.into());
    }
    
    // Try without timezone
    if let Ok(dt) = DateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        return Ok(dt.into());
    }
    
    Err(format!("Failed to parse datetime: {}", datetime_str))
}

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
