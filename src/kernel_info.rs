use std::fs;
use std::io;
use std::path::Path;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct KernelInfo {
    pub version: String,
    pub release: String,
    pub arch: String,
    pub path: String,
}

pub fn get_kernel_version_from_entry(entry_name: &str) -> Option<KernelInfo> {
    // Try to extract kernel version from entry name
    // Entry names often contain kernel version like "Ubuntu, with Linux 5.15.0-91-generic"
    let version_re = Regex::new(r"(\d+\.\d+\.\d+[-\w]*)").ok()?;
    if let Some(caps) = version_re.captures(entry_name) {
        let version = caps.get(1)?.as_str().to_string();
        
        // Try to find corresponding vmlinuz file
        let boot_dir = Path::new("/boot");
        if let Ok(entries) = fs::read_dir(boot_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("vmlinuz-") && file_name.contains(&version) {
                        let release = version.clone();
                        let arch = "x86_64".to_string(); // Default, could be improved
                        return Some(KernelInfo {
                            version: version.clone(),
                            release,
                            arch,
                            path: path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
    }
    None
}

pub fn list_kernel_files() -> Vec<KernelInfo> {
    let mut kernels = Vec::new();
    let boot_dir = Path::new("/boot");
    
    if let Ok(entries) = fs::read_dir(boot_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with("vmlinuz-") && !file_name.contains("old") {
                    let version = file_name.strip_prefix("vmlinuz-").unwrap_or(file_name).to_string();
                    let arch = "x86_64".to_string(); // Could be improved by checking actual arch
                    kernels.push(KernelInfo {
                        version: version.clone(),
                        release: version,
                        arch,
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }
    
    // Sort by version (newest first)
    kernels.sort_by(|a, b| b.version.cmp(&a.version));
    kernels
}

pub fn get_current_kernel() -> Option<String> {
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

pub fn is_kernel_in_use(kernel_version: &str) -> bool {
    if let Some(current) = get_current_kernel() {
        return current == kernel_version;
    }
    false
}

