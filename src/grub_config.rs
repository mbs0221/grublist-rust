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

use crate::colorprint;

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
    
    loop {
        print!("\x1b[2J\x1b[H"); // clear screen
        println!("{}", bcolors.okgreen("╔═══════════════════════════════════════════════════╗"));
        println!("{}", bcolors.okgreen("║     Configure Kernel Boot Parameters              ║"));
        println!("{}", bcolors.okgreen("╚═══════════════════════════════════════════════════╝"));
        println!();
        
        println!("{}Current settings:{}", bcolors.bold(), bcolors.endc());
        println!("  GRUB_CMDLINE_LINUX: {}", bcolors.okblue(&config.grub_cmdline_linux));
        println!("  GRUB_CMDLINE_LINUX_DEFAULT: {}", bcolors.okblue(&config.grub_cmdline_linux_default));
        println!();
        println!("{}Options:{}", bcolors.bold(), bcolors.endc());
        println!("  1. Edit GRUB_CMDLINE_LINUX");
        println!("  2. Edit GRUB_CMDLINE_LINUX_DEFAULT");
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
                print!("{}Enter GRUB_CMDLINE_LINUX (current: {}): {}", 
                       bcolors.bold(), 
                       bcolors.okblue(&config.grub_cmdline_linux),
                       bcolors.endc());
                io::stdout().flush().unwrap();
                let mut new_value = String::new();
                if io::stdin().read_line(&mut new_value).is_ok() {
                    config.grub_cmdline_linux = new_value.trim().to_string();
                }
            }
            "2" => {
                print!("{}Enter GRUB_CMDLINE_LINUX_DEFAULT (current: {}): {}", 
                       bcolors.bold(),
                       bcolors.okblue(&config.grub_cmdline_linux_default),
                       bcolors.endc());
                io::stdout().flush().unwrap();
                let mut new_value = String::new();
                if io::stdin().read_line(&mut new_value).is_ok() {
                    config.grub_cmdline_linux_default = new_value.trim().to_string();
                }
            }
            "3" => {
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

