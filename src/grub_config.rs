use std::fs;
use std::io::{self, BufRead, BufReader};
use std::fs::File;
use regex::Regex;

pub struct GrubConfig {
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
        
        let mut grub_default = String::new();
        let mut grub_cmdline_linux = String::new();
        let mut grub_cmdline_linux_default = String::new();
        let mut grub_timeout = String::new();
        let mut grub_timeout_style = String::new();
        
        let default_re = Regex::new(r#"^\s*GRUB_DEFAULT\s*=\s*(.+)$"#).unwrap();
        let cmdline_linux_re = Regex::new(r#"^\s*GRUB_CMDLINE_LINUX\s*=\s*(.+)$"#).unwrap();
        let cmdline_linux_default_re = Regex::new(r#"^\s*GRUB_CMDLINE_LINUX_DEFAULT\s*=\s*(.+)$"#).unwrap();
        let timeout_re = Regex::new(r#"^\s*GRUB_TIMEOUT\s*=\s*(.+)$"#).unwrap();
        let timeout_style_re = Regex::new(r#"^\s*GRUB_TIMEOUT_STYLE\s*=\s*(.+)$"#).unwrap();
        
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            
            if let Some(caps) = default_re.captures(&line) {
                grub_default = caps.get(1).unwrap().as_str().trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(caps) = cmdline_linux_re.captures(&line) {
                grub_cmdline_linux = caps.get(1).unwrap().as_str().trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(caps) = cmdline_linux_default_re.captures(&line) {
                grub_cmdline_linux_default = caps.get(1).unwrap().as_str().trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(caps) = timeout_re.captures(&line) {
                grub_timeout = caps.get(1).unwrap().as_str().trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(caps) = timeout_style_re.captures(&line) {
                grub_timeout_style = caps.get(1).unwrap().as_str().trim_matches('"').trim_matches('\'').to_string();
            }
        }
        
        // Set defaults if not found
        if grub_timeout.is_empty() {
            grub_timeout = "5".to_string();
        }
        if grub_timeout_style.is_empty() {
            grub_timeout_style = "menu".to_string();
        }
        
        Ok(GrubConfig {
            grub_default,
            grub_cmdline_linux,
            grub_cmdline_linux_default,
            grub_timeout,
            grub_timeout_style,
        })
    }
    
    pub fn save(&self) -> Result<(), String> {
        let content = fs::read_to_string("/etc/default/grub")
            .map_err(|e| format!("Failed to read /etc/default/grub: {}", e))?;
        
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        let default_re = Regex::new(r#"^\s*GRUB_DEFAULT\s*="#).unwrap();
        let cmdline_linux_re = Regex::new(r#"^\s*GRUB_CMDLINE_LINUX\s*="#).unwrap();
        let cmdline_linux_default_re = Regex::new(r#"^\s*GRUB_CMDLINE_LINUX_DEFAULT\s*="#).unwrap();
        let timeout_re = Regex::new(r#"^\s*GRUB_TIMEOUT\s*="#).unwrap();
        let timeout_style_re = Regex::new(r#"^\s*GRUB_TIMEOUT_STYLE\s*="#).unwrap();
        
        let mut found_cmdline_linux = false;
        let mut found_cmdline_linux_default = false;
        let mut found_timeout = false;
        let mut found_timeout_style = false;
        
        for line in &mut lines {
            if default_re.is_match(line) {
                *line = format!("GRUB_DEFAULT={}", self.grub_default);
            } else if cmdline_linux_re.is_match(line) {
                *line = format!("GRUB_CMDLINE_LINUX=\"{}\"", self.grub_cmdline_linux);
                found_cmdline_linux = true;
            } else if cmdline_linux_default_re.is_match(line) {
                *line = format!("GRUB_CMDLINE_LINUX_DEFAULT=\"{}\"", self.grub_cmdline_linux_default);
                found_cmdline_linux_default = true;
            } else if timeout_re.is_match(line) {
                *line = format!("GRUB_TIMEOUT={}", self.grub_timeout);
                found_timeout = true;
            } else if timeout_style_re.is_match(line) {
                *line = format!("GRUB_TIMEOUT_STYLE={}", self.grub_timeout_style);
                found_timeout_style = true;
            }
        }
        
        // Add missing entries if they don't exist
        if !found_cmdline_linux {
            lines.push(format!("GRUB_CMDLINE_LINUX=\"{}\"", self.grub_cmdline_linux));
        }
        if !found_cmdline_linux_default {
            lines.push(format!("GRUB_CMDLINE_LINUX_DEFAULT=\"{}\"", self.grub_cmdline_linux_default));
        }
        if !found_timeout {
            lines.push(format!("GRUB_TIMEOUT={}", self.grub_timeout));
        }
        if !found_timeout_style {
            lines.push(format!("GRUB_TIMEOUT_STYLE={}", self.grub_timeout_style));
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

fn parse_parameters(cmdline: &str) -> Vec<String> {
    if cmdline.trim().is_empty() {
        return Vec::new();
    }
    cmdline.split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

fn join_parameters(params: &[String]) -> String {
    params.join(" ")
}

use crate::colorprint;

fn split_parameter(param: &str) -> (String, Option<String>) {
    if let Some(pos) = param.find('=') {
        let name = param[..pos].to_string();
        let value = param[pos + 1..].to_string();
        (name, Some(value))
    } else {
        (param.to_string(), None)
    }
}

fn format_parameter(name: &str, value: Option<&str>) -> String {
    if let Some(val) = value {
        format!("{}={}", name, val)
    } else {
        name.to_string()
    }
}

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
                    bcolors.warning().to_string() + "empty" + bcolors.endc()
                } else { 
                    format!("{} parameters", linux_params.len())
                });
        println!("  2. GRUB_CMDLINE_LINUX_DEFAULT ({})", 
                if linux_default_params.is_empty() { 
                    bcolors.warning().to_string() + "empty" + bcolors.endc()
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
    use std::io::{self, Write};
    
    let config = match GrubConfig::load() {
        Ok(c) => c,
        Err(e) => {
            println!("{}Error loading config: {}{}", bcolors.fail(""), e, bcolors.endc());
            println!("\nPress Enter to continue...");
            let _ = io::stdin().read_line(&mut String::new());
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
        }
    }
    
    println!();
    println!("Press Enter to continue...");
    let _ = io::stdin().read_line(&mut String::new());
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

