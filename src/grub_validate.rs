use std::process::Command;
use std::io;
use regex::Regex;

pub fn validate_grub_config() -> Result<ValidationResult, String> {
    // Try to run grub-mkconfig --dry-run
    let output = Command::new("grub-mkconfig")
        .arg("--dry-run")
        .output()
        .map_err(|e| format!("Failed to run grub-mkconfig: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    let success = output.status.success();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    
    // Parse output for errors and warnings
    for line in stdout.lines().chain(stderr.lines()) {
        if line.to_lowercase().contains("error") {
            errors.push(line.to_string());
        } else if line.to_lowercase().contains("warning") {
            warnings.push(line.to_string());
        }
    }
    
    Ok(ValidationResult {
        valid: success && errors.is_empty(),
        errors,
        warnings,
        output: stdout.to_string(),
    })
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub output: String,
}

/// Get GRUB version (major.minor format, e.g., "2.06" or "2.00")
pub fn get_grub_version() -> Option<(u32, u32)> {
    // Try grub-mkconfig --version first
    if let Ok(output) = Command::new("grub-mkconfig").arg("--version").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let version_re = Regex::new(r"grub-mkconfig\s+\(GRUB\)\s+(\d+)\.(\d+)").ok()?;
        if let Some(caps) = version_re.captures(&stdout) {
            if let (Ok(major), Ok(minor)) = (
                caps.get(1)?.as_str().parse::<u32>(),
                caps.get(2)?.as_str().parse::<u32>(),
            ) {
                return Some((major, minor));
            }
        }
    }
    
    // Try grub-install --version as fallback
    if let Ok(output) = Command::new("grub-install").arg("--version").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let version_re = Regex::new(r"grub-install\s+\(GRUB\)\s+(\d+)\.(\d+)").ok()?;
        if let Some(caps) = version_re.captures(&stdout) {
            if let (Ok(major), Ok(minor)) = (
                caps.get(1)?.as_str().parse::<u32>(),
                caps.get(2)?.as_str().parse::<u32>(),
            ) {
                return Some((major, minor));
            }
        }
    }
    
    None
}

/// Check if GRUB_DEFAULT uses old title format
/// Returns true if it's an old format that needs to be converted
pub fn is_old_grub_default_format(value: &str) -> bool {
    let trimmed = value.trim_matches('"').trim_matches('\'');
    
    // Old format: just a title like "Ubuntu, with Linux 6.5.0-rc2-snp-host-ec25de0e7141"
    // New format: either numeric path (0>2), menu title path, or UUID format
    
    // Check if it's a numeric path (new format)
    if trimmed.split('>').all(|s| s.parse::<usize>().is_ok()) {
        return false;
    }
    
    // Check if it's UUID format (starts with gnulinux-)
    if trimmed.starts_with("gnulinux-") {
        return false;
    }
    
    // Check if it's "saved" (valid format)
    if trimmed == "saved" {
        return false;
    }
    
    // Check if it contains ">" and looks like a menu title path (new format for < 2.00)
    if trimmed.contains('>') {
        // This might be a menu title path, which is valid for GRUB < 2.00
        return false;
    }
    
    // If it's a single title without ">" and not "saved", it's likely old format
    // But we need to be careful - it could be a valid single-level entry
    // The warning message suggests it's old format if it's just a title
    // We'll check if it looks like a kernel title (contains "Linux" and version numbers)
    let kernel_title_re = Regex::new(r".*Linux\s+[\d\.-]+.*").ok();
    if let Some(re) = kernel_title_re {
        if re.is_match(trimmed) && !trimmed.contains('>') {
            return true;
        }
    }
    
    false
}

/// Detect and fix old GRUB_DEFAULT format
/// This function tries to convert old title format to numeric path format
pub fn fix_old_grub_default_format(
    old_value: &str,
    grub_entry: &crate::grub::Entry,
) -> Option<String> {
    let trimmed = old_value.trim_matches('"').trim_matches('\'');
    
    // Try to find the entry by name in the GRUB tree
    fn find_entry_by_name(
        entry: &crate::grub::Entry,
        target_name: &str,
        current_path: &mut Vec<usize>,
    ) -> Option<Vec<usize>> {
        // Check if current entry matches
        if entry.name == target_name {
            return Some(current_path.clone());
        }
        
        // Search in children
        for (idx, child) in entry.children.iter().enumerate() {
            current_path.push(idx);
            if let Some(path) = find_entry_by_name(child, target_name, current_path) {
                return Some(path);
            }
            current_path.pop();
        }
        
        None
    }
    
    let mut path = Vec::new();
    if let Some(found_path) = find_entry_by_name(grub_entry, trimmed, &mut path) {
        // Convert path to string format (e.g., [0, 2] -> "0>2")
        let path_str = found_path
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(">");
        return Some(path_str);
    }
    
    None
}

