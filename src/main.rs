mod colorprint;
mod interaction;
mod grub;
mod grub_config;

use colorprint::Bcolors;
use interaction::get_key_input;
use grub::{Entry, EntryType, load_grub};

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
    
    loop {
        print!("\x1b[2J\x1b[H"); // clear screen
        print_banner(bcolors);
        println!();
        print_entry(&entry, &path, 0, bcolors);
        
        let k = loop {
            match get_key_input() {
                0 => continue,
                key => break key,
            }
        };
        
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
                    // Include config option in max index
                    let max_idx = entry_ref.children.len();
                    let new_p = (p + 1).min(max_idx);
                    path.push(new_p);
                }
            }
            3 | 5 => { // Right & Enter
                // Check if config option is selected
                if path.len() == 1 && path[0] == entry.children.len() {
                    grub_config::edit_kernel_parameters(bcolors);
                    continue;
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
    
    // Add configuration option at root level
    if level == 0 {
        let config_selected = path.len() == 1 && path[0] == root.children.len();
        let indent = " ".repeat(4 * level);
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
    }
}

fn get_entry<'a>(root: &'a Entry, path: &[usize]) -> &'a Entry {
    let mut e = root;
    for &idx in path {
        e = &e.children[idx];
    }
    e
}

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

