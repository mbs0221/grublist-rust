use std::fs;
use std::io;
use std::path::Path;
use crate::kernel_info::{list_kernel_files, get_current_kernel, is_kernel_in_use};

#[derive(Debug, Clone)]
pub struct KernelToClean {
    pub version: String,
    pub files: Vec<String>,
    pub size: u64,
    pub in_use: bool,
}

pub fn scan_unused_kernels() -> Vec<KernelToClean> {
    let mut kernels_to_clean = Vec::new();
    let current_kernel = get_current_kernel();
    
    for kernel_info in list_kernel_files() {
        let version = kernel_info.version.clone();
        let in_use = current_kernel.as_ref()
            .map(|k| k == &version)
            .unwrap_or(false);
        
        if !in_use {
            let mut files = Vec::new();
            let mut total_size = 0u64;
            
            // Find all files related to this kernel
            let boot_dir = Path::new("/boot");
            if let Ok(entries) = fs::read_dir(boot_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.contains(&version) {
                            if let Ok(metadata) = fs::metadata(&path) {
                                total_size += metadata.len();
                                files.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
            
            kernels_to_clean.push(KernelToClean {
                version,
                files,
                size: total_size,
                in_use,
            });
        }
    }
    
    kernels_to_clean.sort_by(|a, b| a.version.cmp(&b.version)); // Oldest first
    kernels_to_clean
}

pub fn delete_kernel_files(kernel_version: &str) -> io::Result<()> {
    let boot_dir = Path::new("/boot");
    
    if let Ok(entries) = fs::read_dir(boot_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.contains(kernel_version) {
                    if path.is_file() {
                        fs::remove_file(&path)?;
                    } else if path.is_dir() {
                        fs::remove_dir_all(&path)?;
                    }
                }
            }
        }
    }
    
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

