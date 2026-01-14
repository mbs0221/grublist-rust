use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
}

pub fn list_backups() -> Vec<BackupInfo> {
    let mut backups = Vec::new();
    let config_dir = Path::new("/etc/default");
    
    if let Ok(entries) = fs::read_dir(config_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with("grub") && file_name.ends_with(".bak") {
                    if let Ok(metadata) = fs::metadata(&path) {
                        backups.push(BackupInfo {
                            path: path.clone(),
                            size: metadata.len(),
                            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                        });
                    }
                }
            }
        }
    }
    
    // Sort by modification time (newest first)
    backups.sort_by(|a, b| b.modified.cmp(&a.modified));
    backups
}

pub fn restore_backup(backup_path: &Path) -> io::Result<()> {
    let target = Path::new("/etc/default/grub");
    
    // Create a new backup of current config before restoring
    if target.exists() {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_backup = format!("/etc/default/grub.pre-restore-{}.bak", timestamp);
        fs::copy(target, &new_backup)?;
    }
    
    fs::copy(backup_path, target)?;
    Ok(())
}

pub fn delete_backup(backup_path: &Path) -> io::Result<()> {
    fs::remove_file(backup_path)?;
    Ok(())
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_idx])
}

pub fn format_time(time: SystemTime) -> String {
    if let Ok(duration) = time.duration_since(SystemTime::UNIX_EPOCH) {
        use chrono::{DateTime, Local, TimeZone};
        if let Some(datetime) = Local.timestamp_opt(duration.as_secs() as i64, 0).single() {
            return datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }
    "Unknown".to_string()
}

