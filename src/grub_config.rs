use std::fs;
use std::io::{self, BufRead, BufReader};
use std::fs::File;
use regex::Regex;

pub struct GrubConfig {
    pub grub_default: String,
    pub grub_cmdline_linux: String,
    pub grub_cmdline_linux_default: String,
}

impl GrubConfig {
    pub fn load() -> Result<Self, String> {
        let file = File::open("/etc/default/grub")
            .map_err(|_| "Failed to open /etc/default/grub".to_string())?;
        
        let mut grub_default = String::new();
        let mut grub_cmdline_linux = String::new();
        let mut grub_cmdline_linux_default = String::new();
        
        let default_re = Regex::new(r#"^\s*GRUB_DEFAULT\s*=\s*(.+)$"#).unwrap();
        let cmdline_linux_re = Regex::new(r#"^\s*GRUB_CMDLINE_LINUX\s*=\s*(.+)$"#).unwrap();
        let cmdline_linux_default_re = Regex::new(r#"^\s*GRUB_CMDLINE_LINUX_DEFAULT\s*=\s*(.+)$"#).unwrap();
        
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
            
            if let Some(caps) = default_re.captures(&line) {
                grub_default = caps.get(1).unwrap().as_str().trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(caps) = cmdline_linux_re.captures(&line) {
                grub_cmdline_linux = caps.get(1).unwrap().as_str().trim_matches('"').trim_matches('\'').to_string();
            } else if let Some(caps) = cmdline_linux_default_re.captures(&line) {
                grub_cmdline_linux_default = caps.get(1).unwrap().as_str().trim_matches('"').trim_matches('\'').to_string();
            }
        }
        
        Ok(GrubConfig {
            grub_default,
            grub_cmdline_linux,
            grub_cmdline_linux_default,
        })
    }
    
    pub fn save(&self) -> Result<(), String> {
        let content = fs::read_to_string("/etc/default/grub")
            .map_err(|e| format!("Failed to read /etc/default/grub: {}", e))?;
        
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        let default_re = Regex::new(r#"^\s*GRUB_DEFAULT\s*="#).unwrap();
        let cmdline_linux_re = Regex::new(r#"^\s*GRUB_CMDLINE_LINUX\s*="#).unwrap();
        let cmdline_linux_default_re = Regex::new(r#"^\s*GRUB_CMDLINE_LINUX_DEFAULT\s*="#).unwrap();
        
        let mut found_cmdline_linux = false;
        let mut found_cmdline_linux_default = false;
        
        for line in &mut lines {
            if default_re.is_match(line) {
                *line = format!("GRUB_DEFAULT={}", self.grub_default);
            } else if cmdline_linux_re.is_match(line) {
                *line = format!("GRUB_CMDLINE_LINUX=\"{}\"", self.grub_cmdline_linux);
                found_cmdline_linux = true;
            } else if cmdline_linux_default_re.is_match(line) {
                *line = format!("GRUB_CMDLINE_LINUX_DEFAULT=\"{}\"", self.grub_cmdline_linux_default);
                found_cmdline_linux_default = true;
            }
        }
        
        // Add missing entries if they don't exist
        if !found_cmdline_linux {
            lines.push(format!("GRUB_CMDLINE_LINUX=\"{}\"", self.grub_cmdline_linux));
        }
        if !found_cmdline_linux_default {
            lines.push(format!("GRUB_CMDLINE_LINUX_DEFAULT=\"{}\"", self.grub_cmdline_linux_default));
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
                println!("  {}. {}", i + 1, bcolors.okblue(param));
            }
            println!();
        }
        
        println!("{}Options:{}", bcolors.bold(), bcolors.endc());
        println!("  1-{}. Edit parameter", params.len());
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
                print!("{}Enter new value for parameter {} (current: {}): {}", 
                       bcolors.bold(),
                       idx,
                       bcolors.okblue(&params[idx - 1]),
                       bcolors.endc());
                io::stdout().flush().unwrap();
                let mut new_value = String::new();
                if io::stdin().read_line(&mut new_value).is_ok() {
                    let trimmed = new_value.trim();
                    if !trimmed.is_empty() {
                        params[idx - 1] = trimmed.to_string();
                    }
                }
            }
        } else if choice == "a" {
            print!("{}Enter new parameter: {}", bcolors.bold(), bcolors.endc());
            io::stdout().flush().unwrap();
            let mut new_param = String::new();
            if io::stdin().read_line(&mut new_param).is_ok() {
                let trimmed = new_param.trim();
                if !trimmed.is_empty() {
                    params.push(trimmed.to_string());
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

