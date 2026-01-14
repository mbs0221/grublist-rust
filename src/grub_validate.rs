use std::process::Command;
use std::io;

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

