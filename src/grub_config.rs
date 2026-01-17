use std::fs;
use std::io::{BufRead, BufReader};
use std::fs::File;
use std::collections::HashMap;
use regex::Regex;

pub struct GrubConfig {
    pub params: HashMap<String, String>,
    // Keep these for backward compatibility
    pub grub_default: String,
    pub grub_cmdline_linux: String,
    pub grub_cmdline_linux_default: String,
    pub grub_timeout: String,
    pub grub_timeout_style: String,
}

impl GrubConfig {
    pub fn load() -> Result<Self, String> {
        let file = File::open("/etc/default/grub")
            .map_err(|_| "Failed to open /etc/default/grub".to_string())?;
        
        let mut params = HashMap::new();
        let param_re = Regex::new(r#"^\s*([A-Z_][A-Z0-9_]*)\s*=\s*(.+)$"#).unwrap();
        
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            let line_trimmed = line.trim();
            
            // Skip empty lines and comments
            if line_trimmed.is_empty() || line_trimmed.starts_with('#') {
                continue;
            }
            
            // Parse all GRUB parameters
            if let Some(caps) = param_re.captures(&line_trimmed) {
                let key = caps.get(1).unwrap().as_str().to_string();
                let mut value = caps.get(2).unwrap().as_str().to_string();
                
                // Remove quotes if present
                value = value.trim_matches('"').trim_matches('\'').trim().to_string();
                
                params.insert(key, value);
            }
        }
        
        // Set defaults if not found
        if !params.contains_key("GRUB_TIMEOUT") {
            params.insert("GRUB_TIMEOUT".to_string(), "5".to_string());
        }
        if !params.contains_key("GRUB_TIMEOUT_STYLE") {
            params.insert("GRUB_TIMEOUT_STYLE".to_string(), "menu".to_string());
        }
        
        // Extract commonly used parameters for backward compatibility
        let mut grub_default = params.get("GRUB_DEFAULT").cloned().unwrap_or_default();
        
        // Check if GRUB_DEFAULT uses old format and warn
        if !grub_default.is_empty() && grub_default != "saved" {
            if crate::grub_validate::is_old_grub_default_format(&grub_default) {
                // Try to load GRUB entries and fix the format
                if let Some(grub_entry) = crate::grub::load_grub() {
                    if let Some(fixed_value) = crate::grub_validate::fix_old_grub_default_format(&grub_default, &grub_entry) {
                        // Update the value in params
                        params.insert("GRUB_DEFAULT".to_string(), fixed_value.clone());
                        grub_default = fixed_value;
                    }
                }
            }
        }
        
        let grub_cmdline_linux = params.get("GRUB_CMDLINE_LINUX").cloned().unwrap_or_default();
        let grub_cmdline_linux_default = params.get("GRUB_CMDLINE_LINUX_DEFAULT").cloned().unwrap_or_default();
        let grub_timeout = params.get("GRUB_TIMEOUT").cloned().unwrap_or_else(|| "5".to_string());
        let grub_timeout_style = params.get("GRUB_TIMEOUT_STYLE").cloned().unwrap_or_else(|| "menu".to_string());
        
        Ok(GrubConfig {
            params,
            grub_default,
            grub_cmdline_linux,
            grub_cmdline_linux_default,
            grub_timeout,
            grub_timeout_style,
        })
    }
    
    /// Validate and fix GRUB_DEFAULT format if needed
    /// Returns true if the value was fixed
    pub fn validate_and_fix_grub_default(&mut self, grub_entry: &crate::grub::Entry) -> bool {
        if self.grub_default.is_empty() || self.grub_default == "saved" {
            return false;
        }
        
        if crate::grub_validate::is_old_grub_default_format(&self.grub_default) {
            if let Some(fixed_value) = crate::grub_validate::fix_old_grub_default_format(&self.grub_default, grub_entry) {
                self.set("GRUB_DEFAULT", fixed_value);
                return true;
            }
        }
        
        false
    }
    
    pub fn get(&self, key: &str) -> Option<&String> {
        self.params.get(key)
    }
    
    pub fn set(&mut self, key: &str, value: String) {
        self.params.insert(key.to_string(), value.clone());
        
        // Update backward compatibility fields
        match key {
            "GRUB_DEFAULT" => self.grub_default = value,
            "GRUB_CMDLINE_LINUX" => self.grub_cmdline_linux = value,
            "GRUB_CMDLINE_LINUX_DEFAULT" => self.grub_cmdline_linux_default = value,
            "GRUB_TIMEOUT" => self.grub_timeout = value,
            "GRUB_TIMEOUT_STYLE" => self.grub_timeout_style = value,
            _ => {}
        }
    }
    
    pub fn get_all_params(&self) -> &HashMap<String, String> {
        &self.params
    }
    
    pub fn save(&self) -> Result<(), String> {
        let content = fs::read_to_string("/etc/default/grub")
            .map_err(|e| format!("Failed to read /etc/default/grub: {}", e))?;
        
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let param_re = Regex::new(r#"^\s*([A-Z_][A-Z0-9_]*)\s*="#).unwrap();
        let mut found_params: std::collections::HashSet<String> = std::collections::HashSet::new();
        
        // Update existing parameters
        for line in &mut lines {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() || line_trimmed.starts_with('#') {
                continue;
            }
            
            if let Some(caps) = param_re.captures(&line_trimmed) {
                let key = caps.get(1).unwrap().as_str();
                if let Some(value) = self.params.get(key) {
                    found_params.insert(key.to_string());
                    // Determine if value needs quotes (for CMDLINE parameters)
                    if key == "GRUB_CMDLINE_LINUX" || key == "GRUB_CMDLINE_LINUX_DEFAULT" {
                        *line = format!("{}=\"{}\"", key, value);
                    } else {
                        *line = format!("{}={}", key, value);
                    }
                }
            }
        }
        
        // Add missing parameters at the end
        for (key, value) in &self.params {
            if !found_params.contains(key) {
                if key == "GRUB_CMDLINE_LINUX" || key == "GRUB_CMDLINE_LINUX_DEFAULT" {
                    lines.push(format!("{}=\"{}\"", key, value));
                } else {
                    lines.push(format!("{}={}", key, value));
                }
            }
        }
        
        let new_content = lines.join("\n") + "\n";
        
        // Create backup
        fs::copy("/etc/default/grub", "/etc/default/grub.bak")
            .map_err(|e| format!("Failed to create backup: {}", e))?;
        
        fs::write("/etc/default/grub", new_content)
            .map_err(|e| format!("Failed to write /etc/default/grub: {}", e))?;
        
        Ok(())
    }
}

pub fn parse_parameters(cmdline: &str) -> Vec<String> {
    if cmdline.trim().is_empty() {
        return Vec::new();
    }
    cmdline.split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

pub fn join_parameters(params: &[String]) -> String {
    params.join(" ")
}

pub fn split_parameter(param: &str) -> (String, Option<String>) {
    if let Some(pos) = param.find('=') {
        let name = param[..pos].to_string();
        let value = param[pos + 1..].to_string();
        (name, Some(value))
    } else {
        (param.to_string(), None)
    }
}

pub fn format_parameter(name: &str, value: Option<&str>) -> String {
    if let Some(val) = value {
        format!("{}={}", name, val)
    } else {
        name.to_string()
    }
}

use crate::colorprint;

fn edit_parameter_list(title: &str, params: &mut Vec<String>, bcolors: &colorprint::Bcolors) -> bool {
    use std::io::{self, Write};
    
    loop {
        print!("\x1b[2J\x1b[H"); // clear screen
        println!("{}", bcolors.okgreen(&format!("╔═══════════════════════════════════════════════════╗")));
        println!("{}", bcolors.okgreen(&format!("║     {} ║", format!("{:<45}", title))));
        println!("{}", bcolors.okgreen("╚═══════════════════════════════════════════════════╝"));
        println!();
        
        if params.is_empty() {
            println!("{}No parameters configured.{}", bcolors.warning(), bcolors.endc());
            println!();
        } else {
            println!("{}Parameters:{}", bcolors.bold(), bcolors.endc());
            for (i, param) in params.iter().enumerate() {
                let (name, value) = split_parameter(param);
                if let Some(val) = value {
                    println!("  {}. {}={}", 
                            i + 1, 
                            bcolors.okblue(&name),
                            bcolors.okgreen(&val));
                } else {
                    println!("  {}. {}", i + 1, bcolors.okblue(&name));
                }
            }
            println!();
        }
        
        println!("{}Options:{}", bcolors.bold(), bcolors.endc());
        if !params.is_empty() {
            println!("  1-{}. Edit parameter value", params.len());
        }
        println!("  a. Add new parameter");
        if !params.is_empty() {
            println!("  d. Delete parameter");
        }
        println!("  s. Save and continue");
        println!("  c. Cancel");
        println!();
        print!("{}Select option: {}", bcolors.bold(), bcolors.endc());
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        
        let choice = input.trim().to_lowercase();
        
        if let Ok(idx) = choice.parse::<usize>() {
            if idx >= 1 && idx <= params.len() {
                let (name, current_value) = split_parameter(&params[idx - 1]);
                let has_value = current_value.is_some();
                
                if let Some(ref val) = current_value {
                    // Parameter has a value, edit only the value
                    print!("{}Enter new value for {} (current: {}): {}", 
                           bcolors.bold(),
                           bcolors.okblue(&name),
                           bcolors.okgreen(val),
                           bcolors.endc());
                } else {
                    // Parameter has no value, allow adding value or editing name
                    print!("{}Enter value for {} (or leave empty to edit name): {}", 
                           bcolors.bold(),
                           bcolors.okblue(&name),
                           bcolors.endc());
                }
                io::stdout().flush().unwrap();
                let mut new_value = String::new();
                if io::stdin().read_line(&mut new_value).is_ok() {
                    let trimmed = new_value.trim();
                    if trimmed.is_empty() {
                        // If empty and parameter has no value, allow editing the name
                        if !has_value {
                            print!("{}Enter new parameter name (current: {}): {}", 
                                   bcolors.bold(),
                                   bcolors.okblue(&name),
                                   bcolors.endc());
                            io::stdout().flush().unwrap();
                            let mut new_name = String::new();
                            if io::stdin().read_line(&mut new_name).is_ok() {
                                let trimmed_name = new_name.trim();
                                if !trimmed_name.is_empty() {
                                    params[idx - 1] = trimmed_name.to_string();
                                }
                            }
                        }
                    } else {
                        // Update the value
                        params[idx - 1] = format_parameter(&name, Some(trimmed));
                    }
                }
            }
        } else if choice == "a" {
            print!("{}Enter parameter name: {}", bcolors.bold(), bcolors.endc());
            io::stdout().flush().unwrap();
            let mut param_name = String::new();
            if io::stdin().read_line(&mut param_name).is_ok() {
                let trimmed_name = param_name.trim();
                if !trimmed_name.is_empty() {
                    print!("{}Enter parameter value (or leave empty for flag parameter): {}", 
                           bcolors.bold(), bcolors.endc());
                    io::stdout().flush().unwrap();
                    let mut param_value = String::new();
                    if io::stdin().read_line(&mut param_value).is_ok() {
                        let trimmed_value = param_value.trim();
                        if trimmed_value.is_empty() {
                            params.push(trimmed_name.to_string());
                        } else {
                            params.push(format_parameter(trimmed_name, Some(trimmed_value)));
                        }
                    }
                }
            }
        } else if choice == "d" && !params.is_empty() {
            print!("{}Enter parameter number to delete [1-{}]: ", 
                   bcolors.bold(), 
                   params.len());
            io::stdout().flush().unwrap();
            let mut del_input = String::new();
            if io::stdin().read_line(&mut del_input).is_ok() {
                if let Ok(del_idx) = del_input.trim().parse::<usize>() {
                    if del_idx >= 1 && del_idx <= params.len() {
                        params.remove(del_idx - 1);
                    }
                }
            }
        } else if choice == "s" {
            return true;
        } else if choice == "c" {
            return false;
        }
    }
}

pub fn edit_kernel_parameters(bcolors: &colorprint::Bcolors) -> bool {
    use std::io::{self, Write};
    
    print!("\x1b[2J\x1b[H"); // clear screen
    
    let mut config = match GrubConfig::load() {
        Ok(c) => c,
        Err(e) => {
            println!("{}Error: {}{}", bcolors.fail(""), e, bcolors.endc());
            println!("\nPress Enter to continue...");
            let _ = io::stdin().read_line(&mut String::new());
            return false;
        }
    };
    
    let mut linux_params = parse_parameters(&config.grub_cmdline_linux);
    let mut linux_default_params = parse_parameters(&config.grub_cmdline_linux_default);
    
    loop {
        print!("\x1b[2J\x1b[H"); // clear screen
        println!("{}", bcolors.okgreen("╔═══════════════════════════════════════════════════╗"));
        println!("{}", bcolors.okgreen("║     Configure Kernel Boot Parameters              ║"));
        println!("{}", bcolors.okgreen("╚═══════════════════════════════════════════════════╝"));
        println!();
        
        println!("{}Select configuration to edit:{}", bcolors.bold(), bcolors.endc());
        println!("  1. GRUB_CMDLINE_LINUX ({})", 
                if linux_params.is_empty() { 
                    format!("{}{}{}", bcolors.warning(), "empty", bcolors.endc())
                } else { 
                    format!("{} parameters", linux_params.len())
                });
        println!("  2. GRUB_CMDLINE_LINUX_DEFAULT ({})", 
                if linux_default_params.is_empty() { 
                    format!("{}{}{}", bcolors.warning(), "empty", bcolors.endc())
                } else { 
                    format!("{} parameters", linux_default_params.len())
                });
        println!("  3. Save and exit");
        println!("  4. Cancel");
        println!();
        print!("{}Select option [1-4]: {}", bcolors.bold(), bcolors.endc());
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        
        match input.trim() {
            "1" => {
                if edit_parameter_list("Edit GRUB_CMDLINE_LINUX", &mut linux_params, bcolors) {
                    config.grub_cmdline_linux = join_parameters(&linux_params);
                }
            }
            "2" => {
                if edit_parameter_list("Edit GRUB_CMDLINE_LINUX_DEFAULT", &mut linux_default_params, bcolors) {
                    config.grub_cmdline_linux_default = join_parameters(&linux_default_params);
                }
            }
            "3" => {
                config.grub_cmdline_linux = join_parameters(&linux_params);
                config.grub_cmdline_linux_default = join_parameters(&linux_default_params);
                
                match config.save() {
                    Ok(_) => {
                        println!();
                        println!("{}Configuration saved successfully!{}", 
                                bcolors.okgreen(""), bcolors.endc());
                        println!();
                        println!("{}Please run the following command to apply changes:{}",
                                bcolors.warning(), bcolors.endc());
                        println!("  {}{}{}", bcolors.bold(), "sudo update-grub", bcolors.endc());
                        println!();
                        println!("Press Enter to continue...");
                        let _ = io::stdin().read_line(&mut String::new());
                        return true;
                    }
                    Err(e) => {
                        println!();
                        println!("{}Error saving configuration: {}{}", 
                                bcolors.fail(""), e, bcolors.endc());
                        println!();
                        println!("Press Enter to continue...");
                        let _ = io::stdin().read_line(&mut String::new());
                    }
                }
            }
            "4" => {
                return false;
            }
            _ => {
                continue;
            }
        }
    }
}

// Function 1: Set default boot entry (permanent)
pub fn set_default_entry_interactive(entry: &crate::grub::Entry, bcolors: &colorprint::Bcolors) -> bool {
    use std::io::{self, Write};
    
    print!("\x1b[2J\x1b[H"); // clear screen
    println!("{}", bcolors.okgreen("╔═══════════════════════════════════════════════════╗"));
    println!("{}", bcolors.okgreen("║     Set Default Boot Entry (Permanent)           ║"));
    println!("{}", bcolors.okgreen("╚═══════════════════════════════════════════════════╝"));
    println!();
    println!("{}Please select the boot entry you want to set as default.{}", 
            bcolors.bold(), bcolors.endc());
    println!("{}Navigate to the entry and press Enter to select it.{}", 
            bcolors.okblue(""), bcolors.endc());
    println!();
    println!("Press Enter to continue...");
    let _ = io::stdin().read_line(&mut String::new());
    false // Return false to continue menu navigation
}

pub fn set_default_entry(entry: &crate::grub::Entry, path: &[usize], bcolors: &colorprint::Bcolors) -> bool {
    use std::io::{self, Write};
    
    let p_str: String = path.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(">");
    
    let entry_ref = crate::grub::get_entry(entry, path);
    
    println!();
    println!("{}Set '{}' as permanent default boot entry?{}", 
            bcolors.bold(), 
            entry_ref.name,
            bcolors.endc());
    println!("{}Path: {}{}", bcolors.okblue(""), p_str, bcolors.endc());
    println!();
    print!("{}Confirm [Y/n]: {}", bcolors.bold(), bcolors.endc());
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let answer = input.trim().to_lowercase();
        if answer == "y" || answer == "yes" || answer.is_empty() {
            let mut config = match GrubConfig::load() {
                Ok(c) => c,
                Err(e) => {
                    println!("{}Error loading config: {}{}", bcolors.fail(""), e, bcolors.endc());
                    println!("\nPress Enter to continue...");
                    let _ = io::stdin().read_line(&mut String::new());
                    return false;
                }
            };
            
            config.grub_default = format!("\"{}\"", p_str);
            
            match config.save() {
                Ok(_) => {
                    println!();
                    println!("{}Default boot entry set successfully!{}", 
                            bcolors.okgreen(""), bcolors.endc());
                    println!();
                    println!("{}Please run: {}{}", 
                            bcolors.warning(), 
                            bcolors.bold(),
                            bcolors.endc());
                    println!("  {}{}{}", bcolors.bold(), "sudo update-grub", bcolors.endc());
                    println!();
                    println!("Press Enter to continue...");
                    let _ = io::stdin().read_line(&mut String::new());
                    return true;
                }
                Err(e) => {
                    println!();
                    println!("{}Error saving configuration: {}{}", 
                            bcolors.fail(""), e, bcolors.endc());
                    println!();
                    println!("Press Enter to continue...");
                    let _ = io::stdin().read_line(&mut String::new());
                    return false;
                }
            }
        }
    }
    false
}

// Function 2: View current default boot entry
pub fn view_default_entry(entry: &crate::grub::Entry, bcolors: &colorprint::Bcolors) {
    use std::io::{Write, stdin};
    
    let mut config = match GrubConfig::load() {
        Ok(c) => c,
        Err(e) => {
            println!("{}Error loading config: {}{}", bcolors.fail(""), e, bcolors.endc());
            println!("\nPress Enter to continue...");
            let _ = stdin().read_line(&mut String::new());
            return;
        }
    };
    
    print!("\x1b[2J\x1b[H"); // clear screen
    println!("{}", bcolors.okgreen("╔═══════════════════════════════════════════════════╗"));
    println!("{}", bcolors.okgreen("║     Current Default Boot Entry                    ║"));
    println!("{}", bcolors.okgreen("╚═══════════════════════════════════════════════════╝"));
    println!();
    
    if config.grub_default == "saved" {
        println!("{}Current setting: {}{}", 
                bcolors.bold(), 
                bcolors.okgreen("GRUB_DEFAULT=saved"),
                bcolors.endc());
        println!();
        println!("This means the last selected entry will be used.");
    } else {
        println!("{}Current setting: GRUB_DEFAULT={}{}", 
                bcolors.bold(), 
                bcolors.okblue(&config.grub_default),
                bcolors.endc());
        println!();
        
        // Check if using old format
        if crate::grub_validate::is_old_grub_default_format(&config.grub_default) {
            println!("{}⚠️  WARNING: Using old title format!{}", 
                    bcolors.warning(), bcolors.endc());
            println!();
            println!("GRUB recommends using numeric path format (e.g., '0>2') instead of");
            println!("old title format (e.g., 'Ubuntu, with Linux 6.5.0-rc2-snp-host-ec25de0e7141').");
            println!();
            println!("{}Would you like to fix this automatically? [Y/n]: {}", 
                    bcolors.bold(), bcolors.endc());
            std::io::stdout().flush().unwrap();
            
            let mut input = String::new();
            if stdin().read_line(&mut input).is_ok() {
                let answer = input.trim().to_lowercase();
                if answer == "y" || answer == "yes" || answer.is_empty() {
                    if config.validate_and_fix_grub_default(entry) {
                        match config.save() {
                            Ok(_) => {
                                println!();
                                println!("{}✓ Fixed! GRUB_DEFAULT has been updated to use numeric path format.{}", 
                                        bcolors.okgreen(""), bcolors.endc());
                                println!();
                                println!("{}Please run: {}{}", 
                                        bcolors.warning(), 
                                        bcolors.bold(),
                                        bcolors.endc());
                                println!("  {}{}{}", bcolors.bold(), "sudo update-grub", bcolors.endc());
                                println!();
                            }
                            Err(e) => {
                                println!();
                                println!("{}Error saving fixed configuration: {}{}", 
                                        bcolors.fail(""), e, bcolors.endc());
                                println!();
                            }
                        }
                    } else {
                        println!();
                        println!("{}Could not automatically fix the format.{}", 
                                bcolors.warning(), bcolors.endc());
                        println!("Please manually update GRUB_DEFAULT in /etc/default/grub");
                        println!();
                    }
                }
            }
        }
        
        // Try to find and display the entry name
        let path_str = config.grub_default.trim_matches('"').trim_matches('\'');
        let path: Result<Vec<usize>, _> = path_str.split('>')
            .map(|s| s.parse::<usize>())
            .collect();
        
        if let Ok(path_vec) = path {
            if let Some(entry_ref) = crate::grub::try_get_entry(entry, &path_vec) {
                println!("{}Entry name: {}{}", 
                        bcolors.bold(),
                        bcolors.okgreen(&entry_ref.name),
                        bcolors.endc());
            } else {
                println!("{}Warning: Could not find entry at path '{}'{}", 
                        bcolors.warning(),
                        path_str,
                        bcolors.endc());
            }
        } else {
            // Not a numeric path, might be old format or other format
            println!("{}Note: Current format is not a numeric path.{}", 
                    bcolors.okblue(""), bcolors.endc());
        }
    }
    
    println!();
    println!("Press Enter to continue...");
    let _ = stdin().read_line(&mut String::new());
}

// Function 3: Configure GRUB timeout
pub fn configure_timeout(bcolors: &colorprint::Bcolors) -> bool {
    use std::io::{self, Write};
    
    let mut config = match GrubConfig::load() {
        Ok(c) => c,
        Err(e) => {
            println!("{}Error loading config: {}{}", bcolors.fail(""), e, bcolors.endc());
            println!("\nPress Enter to continue...");
            let _ = io::stdin().read_line(&mut String::new());
            return false;
        }
    };
    
    loop {
        print!("\x1b[2J\x1b[H"); // clear screen
        println!("{}", bcolors.okgreen("╔═══════════════════════════════════════════════════╗"));
        println!("{}", bcolors.okgreen("║     Configure GRUB Timeout                        ║"));
        println!("{}", bcolors.okgreen("╚═══════════════════════════════════════════════════╝"));
        println!();
        
        println!("{}Current settings:{}", bcolors.bold(), bcolors.endc());
        println!("  GRUB_TIMEOUT: {}", bcolors.okblue(&config.grub_timeout));
        println!("  GRUB_TIMEOUT_STYLE: {}", bcolors.okblue(&config.grub_timeout_style));
        println!();
        
        println!("{}Options:{}", bcolors.bold(), bcolors.endc());
        println!("  1. Set timeout (seconds, -1 for no timeout)");
        println!("  2. Set timeout style (menu/hidden/countdown)");
        println!("  3. Save and exit");
        println!("  4. Cancel");
        println!();
        print!("{}Select option [1-4]: {}", bcolors.bold(), bcolors.endc());
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }
        
        match input.trim() {
            "1" => {
                print!("{}Enter timeout in seconds (current: {}, -1 for no timeout): {}", 
                       bcolors.bold(),
                       bcolors.okblue(&config.grub_timeout),
                       bcolors.endc());
                io::stdout().flush().unwrap();
                let mut new_timeout = String::new();
                if io::stdin().read_line(&mut new_timeout).is_ok() {
                    let trimmed = new_timeout.trim();
                    if !trimmed.is_empty() {
                        config.grub_timeout = trimmed.to_string();
                    }
                }
            }
            "2" => {
                println!("{}Timeout style options:{}", bcolors.bold(), bcolors.endc());
                println!("  menu - Show menu");
                println!("  hidden - Hide menu");
                println!("  countdown - Show countdown");
                println!();
                print!("{}Enter timeout style (current: {}): {}", 
                       bcolors.bold(),
                       bcolors.okblue(&config.grub_timeout_style),
                       bcolors.endc());
                io::stdout().flush().unwrap();
                let mut new_style = String::new();
                if io::stdin().read_line(&mut new_style).is_ok() {
                    let trimmed = new_style.trim();
                    if !trimmed.is_empty() {
                        config.grub_timeout_style = trimmed.to_string();
                    }
                }
            }
            "3" => {
                match config.save() {
                    Ok(_) => {
                        println!();
                        println!("{}Configuration saved successfully!{}", 
                                bcolors.okgreen(""), bcolors.endc());
                        println!();
                        println!("{}Please run: {}{}", 
                                bcolors.warning(), 
                                bcolors.bold(),
                                bcolors.endc());
                        println!("  {}{}{}", bcolors.bold(), "sudo update-grub", bcolors.endc());
                        println!();
                        println!("Press Enter to continue...");
                        let _ = io::stdin().read_line(&mut String::new());
                        return true;
                    }
                    Err(e) => {
                        println!();
                        println!("{}Error saving configuration: {}{}", 
                                bcolors.fail(""), e, bcolors.endc());
                        println!();
                        println!("Press Enter to continue...");
                        let _ = io::stdin().read_line(&mut String::new());
                    }
                }
            }
            "4" => {
                return false;
            }
            _ => {
                continue;
            }
        }
    }
}

