mod colorprint;
mod interaction;
mod grub;
mod grub_config;

use colorprint::Bcolors;
use interaction::get_key_input;
use grub::{Entry, EntryType, load_grub, get_entry};
use std::io::{self, Write};

fn main() {
    let bcolors = Bcolors::new();
    
    // Show banner
    print_banner(&bcolors);
    
    let entry = match load_grub() {
        Some(e) => e,
        None => {
            eprintln!("LoadGrub Failed. \"/boot/grub/grub.cfg\" Not Found.");
            return;
        }
    };
    
    menu(entry, &bcolors);
}

fn print_banner(bcolors: &Bcolors) {
    let bold = bcolors.bold();
    let endc = bcolors.endc();
    println!("{}", bcolors.okgreen(&format!(r#"
    ╔═══════════════════════════════════════════════════╗
    ║                                                   ║
    ║            {}GRUBLIST{} v0.1.0                        ║
    ║                                                   ║
    ║     Interactive GRUB Boot Menu Selector           ║
    ║                                                   ║
    ╚═══════════════════════════════════════════════════╝
    "#, bold, endc)));
    println!("{}Controls: ↑↓ Navigate  →/Enter Select  ← Back  q Quit{}\n", 
             bcolors.okblue(""), bcolors.endc());
}

fn menu(entry: Entry, bcolors: &Bcolors) {
    let mut path = vec![0];
    let mut search_mode = false;
    let mut search_query = String::new();
    
    loop {
        print!("\x1b[2J\x1b[H"); // clear screen
        print_banner(bcolors);
        
        if search_mode {
            println!("{}Search mode (press ESC to cancel): {}{}", 
                    bcolors.okblue(""), 
                    bcolors.bold(),
                    bcolors.endc());
            print!("{}Search: {}{}", bcolors.bold(), search_query, bcolors.endc());
            io::stdout().flush().unwrap();
        } else {
            println!();
        }
        
        if search_mode {
            print_entry_with_search(&entry, &path, 0, bcolors, &search_query);
        } else {
            print_entry(&entry, &path, 0, bcolors);
        }
        
        let k = loop {
            match get_key_input() {
                0 => continue,
                key => break key,
            }
        };
        
        if search_mode {
            match k {
                27 => { // ESC - cancel search
                    search_mode = false;
                    search_query.clear();
                }
                13 | 10 => { // Enter - select first match
                    if let Some(matched_path) = find_first_match(&entry, &search_query) {
                        path = matched_path;
                        search_mode = false;
                        search_query.clear();
                    }
                }
                127 => { // Backspace
                    search_query.pop();
                }
                _ => {
                    // Add character to search query (if printable)
                    if k >= 32 && k <= 126 {
                        search_query.push(k as char);
                    }
                }
            }
            continue;
        }
        
        match k {
            1 => { // Up
                if let Some(p) = path.pop() {
                    let new_p = p.saturating_sub(1);
                    path.push(new_p);
                }
            }
            2 => { // Down
                if let Some(p) = path.pop() {
                    let entry_ref = get_entry(&entry, &path);
                    // Include config options in max index
                    let max_idx = entry_ref.children.len() + 3; // kernel params, default entry, timeout
                    let new_p = (p + 1).min(max_idx);
                    path.push(new_p);
                }
            }
            3 | 5 => { // Right & Enter
                // Check if config options are selected
                let config_idx = entry.children.len();
                if path.len() == 1 {
                    if path[0] == config_idx {
                        grub_config::edit_kernel_parameters(bcolors);
                        continue;
                    } else if path[0] == config_idx + 1 {
                        grub_config::view_default_entry(&entry, bcolors);
                        continue;
            } else if path[0] == config_idx + 2 {
                // Show instructions for setting default entry
                print!("\x1b[2J\x1b[H"); // clear screen
                println!("{}", bcolors.okgreen("╔═══════════════════════════════════════════════════╗"));
                println!("{}", bcolors.okgreen("║     Set Default Boot Entry                        ║"));
                println!("{}", bcolors.okgreen("╚═══════════════════════════════════════════════════╝"));
                println!();
                println!("{}Instructions:{}", bcolors.bold(), bcolors.endc());
                println!("  1. Navigate to the boot entry you want to set as default");
                println!("  2. Press '{}d{}' to set it as permanent default", 
                        bcolors.okblue(""), bcolors.endc());
                println!();
                println!("Press Enter to continue...");
                let _ = io::stdin().read_line(&mut String::new());
                continue;
                    } else if path[0] == config_idx + 3 {
                        grub_config::configure_timeout(bcolors);
                        continue;
                    }
                }
                
                let entry_ref = get_entry(&entry, &path);
                if entry_ref.entry_type == EntryType::Submenu {
                    path.push(0);
                } else {
                    if set_entry(&entry, &path, bcolors) {
                        break;
                    }
                }
            }
            4 => { // Left
                if path.len() > 1 {
                    path.pop();
                }
            }
            47 => { // / - start search
                search_mode = true;
                search_query.clear();
            }
            100 => { // d - set as default
                if path.len() > 0 {
                    grub_config::set_default_entry(&entry, &path, bcolors);
                }
            }
            6 => { // q
                break;
            }
            _ => {}
        }
    }
}

fn print_entry(root: &Entry, path: &[usize], level: usize, bcolors: &Bcolors) {
    for (i, child) in root.children.iter().enumerate() {
        let is_selected = level < path.len() && path[level] == i;
        
        let indent = " ".repeat(4 * level);
        
        if is_selected {
            let tag = match child.entry_type {
                EntryType::Submenu => format!("[{}+] ", bcolors.fail("+")),
                EntryType::MenuEntry => format!("[{}●] ", bcolors.okgreen("●")),
                EntryType::Root => String::new(),
            };
            println!("{}{}{}{}{}", 
                indent, 
                tag,
                bcolors.inverse(&child.name),
                bcolors.endc(),
                "");
            
            // If it's a submenu and there's a deeper level in path, recurse
            if child.entry_type == EntryType::Submenu && level + 1 < path.len() {
                print_entry(child, path, level + 1, bcolors);
            }
        } else {
            let tag = match child.entry_type {
                EntryType::Submenu => format!("[{}+] ", bcolors.fail("+")),
                EntryType::MenuEntry => format!("[{}●] ", bcolors.okgreen("●")),
                EntryType::Root => String::new(),
            };
            println!("{}{}{}", indent, tag, child.name);
        }
    }
    
    // Add configuration options at root level
    if level == 0 {
        let config_idx = root.children.len();
        let indent = " ".repeat(4 * level);
        
        // Option 1: Configure Kernel Parameters
        let config_selected = path.len() == 1 && path[0] == config_idx;
        let config_tag = format!("[{}⚙] ", bcolors.okblue("⚙"));
        if config_selected {
            println!("{}{}{}{}{}", 
                indent,
                config_tag,
                bcolors.inverse("Configure Kernel Parameters"),
                bcolors.endc(),
                "");
        } else {
            println!("{}{}{}", indent, config_tag, "Configure Kernel Parameters");
        }
        
        // Option 2: View Default Entry
        let view_selected = path.len() == 1 && path[0] == config_idx + 1;
        let view_tag = format!("[{}👁] ", bcolors.okblue("👁"));
        if view_selected {
            println!("{}{}{}{}{}", 
                indent,
                view_tag,
                bcolors.inverse("View Default Boot Entry"),
                bcolors.endc(),
                "");
        } else {
            println!("{}{}{}", indent, view_tag, "View Default Boot Entry");
        }
        
        // Option 3: Set Default Entry
        let set_selected = path.len() == 1 && path[0] == config_idx + 2;
        let set_tag = format!("[{}⭐] ", bcolors.okblue("⭐"));
        if set_selected {
            println!("{}{}{}{}{}", 
                indent,
                set_tag,
                bcolors.inverse("Set Default Boot Entry"),
                bcolors.endc(),
                "");
        } else {
            println!("{}{}{}", indent, set_tag, "Set Default Boot Entry");
        }
        
        // Option 4: Configure Timeout
        let timeout_selected = path.len() == 1 && path[0] == config_idx + 3;
        let timeout_tag = format!("[{}⏱] ", bcolors.okblue("⏱"));
        if timeout_selected {
            println!("{}{}{}{}{}", 
                indent,
                timeout_tag,
                bcolors.inverse("Configure GRUB Timeout"),
                bcolors.endc(),
                "");
        } else {
            println!("{}{}{}", indent, timeout_tag, "Configure GRUB Timeout");
        }
    }
}

fn print_entry_with_search(root: &Entry, path: &[usize], level: usize, bcolors: &Bcolors, query: &str) {
    for (i, child) in root.children.iter().enumerate() {
        let is_selected = level < path.len() && path[level] == i;
        let matches = child.name.to_lowercase().contains(&query.to_lowercase());
        
        let indent = " ".repeat(4 * level);
        
        if matches || query.is_empty() {
            if is_selected {
                let tag = match child.entry_type {
                    EntryType::Submenu => format!("[{}+] ", bcolors.fail("+")),
                    EntryType::MenuEntry => format!("[{}●] ", bcolors.okgreen("●")),
                    EntryType::Root => String::new(),
                };
                println!("{}{}{}{}{}", 
                    indent, 
                    tag,
                    bcolors.inverse(&child.name),
                    bcolors.endc(),
                    "");
                
                if child.entry_type == EntryType::Submenu && level + 1 < path.len() {
                    print_entry_with_search(child, path, level + 1, bcolors, query);
                }
            } else {
                let tag = match child.entry_type {
                    EntryType::Submenu => format!("[{}+] ", bcolors.fail("+")),
                    EntryType::MenuEntry => format!("[{}●] ", bcolors.okgreen("●")),
                    EntryType::Root => String::new(),
                };
                if matches && !query.is_empty() {
                    println!("{}{}{}{}{}", indent, tag, bcolors.okgreen(&child.name), bcolors.endc(), "");
                } else {
                    println!("{}{}{}", indent, tag, child.name);
                }
            }
        }
        
        if child.entry_type == EntryType::Submenu && level + 1 < path.len() {
            print_entry_with_search(child, path, level + 1, bcolors, query);
        }
    }
}

fn find_first_match(root: &Entry, query: &str) -> Option<Vec<usize>> {
    fn search_recursive(entry: &Entry, query: &str, path: &mut Vec<usize>) -> Option<Vec<usize>> {
        for (i, child) in entry.children.iter().enumerate() {
            path.push(i);
            if child.name.to_lowercase().contains(&query.to_lowercase()) {
                return Some(path.clone());
            }
            if let Some(result) = search_recursive(child, query, path) {
                return Some(result);
            }
            path.pop();
        }
        None
    }
    
    let mut path = Vec::new();
    search_recursive(root, query, &mut path)
}

// get_entry is now in grub module

fn check_default() -> bool {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    
    let file = match File::open("/etc/default/grub") {
        Ok(f) => f,
        Err(_) => {
            eprintln!("CheckDefault Failed. \"/etc/default/grub\" Not Found.");
            return false;
        }
    };
    
    let re = regex::Regex::new(r#"^\s*GRUB_DEFAULT\s*=\s*['"]?(\w+)['"]?"#).unwrap();
    
    for line in BufReader::new(file).lines() {
        if let Ok(line) = line {
            if let Some(caps) = re.captures(&line) {
                if let Some(value) = caps.get(1) {
                    return value.as_str() == "saved";
                }
            }
        }
    }
    
    false
}

fn reboot(bcolors: &Bcolors) {
    use std::io::{self, Write};
    
    loop {
        print!("{}Reboot now? [Y/n]{}", bcolors.bold(), bcolors.endc());
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let answer = input.trim().to_lowercase();
            if answer == "y" || answer == "yes" || answer.is_empty() {
                std::process::Command::new("sudo")
                    .arg("reboot")
                    .status()
                    .ok();
                return;
            } else if answer == "n" || answer == "no" {
                return;
            }
        }
    }
}

fn set_entry(entry: &Entry, path: &[usize], bcolors: &Bcolors) -> bool {
    use std::io::{self, Write};
    
    println!();
    
    if !check_default() {
        println!("{}Please change the following setting in {}{}{}:{}",
            bcolors.warning(),
            bcolors.endc(),
            bcolors.bold(),
            "/etc/default/grub",
            bcolors.endc()
        );
        println!();
        println!("{}{}{} = {}{}",
            bcolors.bold(),
            "GRUB_DEFAULT",
            bcolors.endc(),
            bcolors.okgreen("saved"),
            bcolors.endc()
        );
        println!();
        println!("And then {}{}{}",
            bcolors.bold(),
            "sudo update-grub",
            bcolors.endc()
        );
        return true;
    }
    
    let p_str: String = path.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(">");
    
    let cmd = format!("sudo grub-reboot \"{}\"", p_str);
    
    loop {
        print!("{}Change the Selected Entry? [Y/n]{}", bcolors.bold(), bcolors.endc());
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let answer = input.trim().to_lowercase();
            if answer == "y" || answer == "yes" || answer.is_empty() {
                println!("{}{}{}", bcolors.okgreen(&cmd), bcolors.endc(), "");
                
                std::process::Command::new("sudo")
                    .arg("grub-reboot")
                    .arg(&p_str)
                    .status()
                    .ok();
                
                reboot(bcolors);
                
                let entry_ref = get_entry(entry, path);
                println!("{}{}{}",
                    bcolors.okgreen("Grub Entry has changed to:"),
                    bcolors.endc(),
                    ""
                );
                println!("{}{}{}",
                    bcolors.bold(),
                    entry_ref.name,
                    bcolors.endc()
                );
                return true;
            } else if answer == "n" || answer == "no" {
                return false;
            }
        }
    }
}

