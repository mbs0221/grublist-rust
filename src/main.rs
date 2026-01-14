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
    println!("{}Controls: ↑↓ Navigate  →/Enter Select  ← Back  / Search  d Set Default  q Quit{}\n", 
             bcolors.okblue(""), bcolors.endc());
}

fn menu(entry: Entry, bcolors: &Bcolors) {
    let mut path = vec![0];
    let mut search_mode = false;
    let mut search_query = String::new();
    let mut search_results: Vec<Vec<usize>> = Vec::new();
    let mut search_result_index = 0;
    
    loop {
        print!("\x1b[2J\x1b[H"); // clear screen
        print_banner(bcolors);
        
        if search_mode {
            println!("{}Search mode (press ESC to cancel){}", 
                    bcolors.okblue(""), 
                    bcolors.endc());
            println!("{}Search: {}{}", bcolors.bold(), search_query, bcolors.endc());
            println!();
            
            // Show search results in a submenu
            if !search_results.is_empty() {
                println!("{}Search Results ({} found):{}", 
                        bcolors.bold(), 
                        search_results.len(),
                        bcolors.endc());
                for (i, result_path) in search_results.iter().enumerate() {
                    let entry_ref = get_entry(&entry, result_path);
                    let indent = "    ";
                    let tag = match entry_ref.entry_type {
                        EntryType::Submenu => format!("[{}+] ", bcolors.fail("+")),
                        EntryType::MenuEntry => format!("[{}●] ", bcolors.okgreen("●")),
                        EntryType::Root => String::new(),
                    };
                    
                    if i == search_result_index {
                        println!("{}{}{}{}{}", 
                                indent,
                                tag,
                                bcolors.inverse(&entry_ref.name),
                                bcolors.endc(),
                                "");
                    } else {
                        println!("{}{}{}", indent, tag, entry_ref.name);
                    }
                }
                println!();
            } else if !search_query.is_empty() {
                println!("{}No matching entries found.{}", 
                        bcolors.warning(), bcolors.endc());
                println!();
            }
        } else {
            println!();
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
                    search_results.clear();
                    search_result_index = 0;
                    path = vec![0];
                }
                5 => { // Enter - select current match
                    if !search_results.is_empty() && search_result_index < search_results.len() {
                        path = search_results[search_result_index].clone();
                        search_mode = false;
                        search_query.clear();
                        search_results.clear();
                        search_result_index = 0;
                    }
                }
                127 => { // Backspace
                    search_query.pop();
                    // Recalculate search results
                    search_results = collect_all_matches(&entry, &search_query);
                    if !search_results.is_empty() {
                        search_result_index = 0.min(search_results.len() - 1);
                    } else {
                        search_result_index = 0;
                    }
                }
                1 => { // Up - navigate to previous match
                    if !search_results.is_empty() {
                        if search_result_index > 0 {
                            search_result_index -= 1;
                        } else {
                            search_result_index = search_results.len() - 1; // Wrap around
                        }
                    }
                }
                2 => { // Down - navigate to next match
                    if !search_results.is_empty() {
                        search_result_index = (search_result_index + 1) % search_results.len();
                    }
                }
                _ => {
                    // Add character to search query (if printable)
                    if k >= 32 && k <= 126 {
                        search_query.push(k as char);
                        // Recalculate search results
                        search_results = collect_all_matches(&entry, &search_query);
                        if !search_results.is_empty() {
                            search_result_index = 0;
                        } else {
                            search_result_index = 0;
                        }
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
                        // Enter boot entry selection mode for setting default
                        if let Some(selected_path) = select_boot_entry(&entry, bcolors) {
                            grub_config::set_default_entry(&entry, &selected_path, bcolors);
                        }
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
                // Auto-select first match if available
                if let Some(matched_path) = find_first_match(&entry, &search_query) {
                    path = matched_path;
                }
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


fn collect_all_matches(root: &Entry, query: &str) -> Vec<Vec<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    
    let mut matches = Vec::new();
    let query_lower = query.to_lowercase();
    
    fn search_recursive(entry: &Entry, query: &str, path: &mut Vec<usize>, matches: &mut Vec<Vec<usize>>) {
        for (i, child) in entry.children.iter().enumerate() {
            path.push(i);
            if child.name.to_lowercase().contains(query) {
                matches.push(path.clone());
            }
            search_recursive(child, query, path, matches);
            path.pop();
        }
    }
    
    let mut path = Vec::new();
    search_recursive(root, &query_lower, &mut path, &mut matches);
    matches
}

fn find_first_match(root: &Entry, query: &str) -> Option<Vec<usize>> {
    if query.is_empty() {
        return Some(vec![0]);
    }
    
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

fn find_next_match(root: &Entry, current_path: &[usize], query: &str) -> Option<Vec<usize>> {
    if query.is_empty() {
        return None;
    }
    
    fn collect_matches(entry: &Entry, query: &str, path: &mut Vec<usize>, matches: &mut Vec<Vec<usize>>) {
        for (i, child) in entry.children.iter().enumerate() {
            path.push(i);
            if child.name.to_lowercase().contains(&query.to_lowercase()) {
                matches.push(path.clone());
            }
            collect_matches(child, query, path, matches);
            path.pop();
        }
    }
    
    let mut matches = Vec::new();
    let mut path = Vec::new();
    collect_matches(root, query, &mut path, &mut matches);
    
    // Find current position and return next
    for (idx, m) in matches.iter().enumerate() {
        if m == current_path {
            if idx + 1 < matches.len() {
                return Some(matches[idx + 1].clone());
            } else {
                return Some(matches[0].clone()); // Wrap around
            }
        }
    }
    
    // If current not found, return first match
    matches.first().cloned()
}

fn find_previous_match(root: &Entry, current_path: &[usize], query: &str) -> Option<Vec<usize>> {
    if query.is_empty() {
        return None;
    }
    
    fn collect_matches(entry: &Entry, query: &str, path: &mut Vec<usize>, matches: &mut Vec<Vec<usize>>) {
        for (i, child) in entry.children.iter().enumerate() {
            path.push(i);
            if child.name.to_lowercase().contains(&query.to_lowercase()) {
                matches.push(path.clone());
            }
            collect_matches(child, query, path, matches);
            path.pop();
        }
    }
    
    let mut matches = Vec::new();
    let mut path = Vec::new();
    collect_matches(root, query, &mut path, &mut matches);
    
    // Find current position and return previous
    for (idx, m) in matches.iter().enumerate() {
        if m == current_path {
            if idx > 0 {
                return Some(matches[idx - 1].clone());
            } else {
                return matches.last().cloned(); // Wrap around
            }
        }
    }
    
    // If current not found, return last match
    matches.last().cloned()
}

// get_entry is now in grub module

fn print_entry_only(root: &Entry, path: &[usize], level: usize, bcolors: &Bcolors) {
    // Only print boot entries, no config options
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
                print_entry_only(child, path, level + 1, bcolors);
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
}

fn select_boot_entry(entry: &Entry, bcolors: &Bcolors) -> Option<Vec<usize>> {
    let mut path = vec![0];
    let mut in_selection_mode = true;
    
    print!("\x1b[2J\x1b[H"); // clear screen
    
    while in_selection_mode {
        print!("\x1b[2J\x1b[H"); // clear screen
        println!("{}", bcolors.okgreen("╔═══════════════════════════════════════════════════╗"));
        println!("{}", bcolors.okgreen("║     Select Boot Entry to Set as Default          ║"));
        println!("{}", bcolors.okgreen("╚═══════════════════════════════════════════════════╝"));
        println!();
        println!("{}Navigate to the boot entry and press Enter to select{}", 
                bcolors.okblue(""), bcolors.endc());
        println!("{}Press ← or q to cancel{}", bcolors.okblue(""), bcolors.endc());
        println!();
        
        print_entry_only(&entry, &path, 0, bcolors);
        
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
                    let max_idx = entry_ref.children.len().saturating_sub(1);
                    let new_p = (p + 1).min(max_idx);
                    path.push(new_p);
                }
            }
            3 | 5 => { // Right & Enter
                let entry_ref = get_entry(&entry, &path);
                if entry_ref.entry_type == EntryType::Submenu {
                    path.push(0);
                } else {
                    // Selected a menu entry, return its path
                    return Some(path.clone());
                }
            }
            4 => { // Left
                if path.len() > 1 {
                    path.pop();
                } else {
                    // Exit selection mode
                    return None;
                }
            }
            6 => { // q - quit
                return None;
            }
            _ => {}
        }
    }
    
    None
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

