use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, Clone, PartialEq)]
pub enum EntryType {
    Root,
    MenuEntry,
    Submenu,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub entry_type: EntryType,
    pub children: Vec<Entry>,
}

impl Entry {
    pub fn new(name: String, entry_type: EntryType) -> Self {
        Entry {
            name,
            entry_type,
            children: Vec::new(),
        }
    }
}

pub fn load_grub() -> Option<Entry> {
    let file = File::open("/boot/grub/grub.cfg").ok()?;
    
    let mut entry = Entry::new("root".to_string(), EntryType::Root);
    let mut level: usize = 0;
    
    let menuentry_re = Regex::new(r"^\s*(menuentry|submenu)\s*'([^']*)'").unwrap();
    let open_brace_re = Regex::new(r"\{\s*$").unwrap();
    let close_brace_re = Regex::new(r"^\s*\}").unwrap();
    
    for line in BufReader::new(file).lines() {
        let line = line.ok()?;
        
        // Check for menuentry or submenu
        if let Some(caps) = menuentry_re.captures(&line) {
            let entry_type_str = caps.get(1)?.as_str();
            let name = caps.get(2)?.as_str();
            
            // Navigate to the correct level (similar to Python: e = entry; for i in range(0, level): e = e['child'][childnum-1])
            // We need to navigate through the tree structure using a stack or mutable references
            // Since we can't easily do mutable references through multiple levels, we'll use indices
            let mut path: Vec<usize> = Vec::new();
            for _ in 0..level {
                path.push(0); // We'll update these indices as we build
            }
            
            // Navigate to the parent entry at the correct level
            let mut current = &mut entry;
            for _ in 0..level {
                if current.children.is_empty() {
                    break;
                }
                let last_idx = current.children.len() - 1;
                current = current.children.get_mut(last_idx)?;
            }
            
            // Create new entry
            let entry_type = match entry_type_str {
                "menuentry" => EntryType::MenuEntry,
                "submenu" => EntryType::Submenu,
                _ => EntryType::MenuEntry,
            };
            
            let new_entry = Entry::new(name.to_string(), entry_type);
            current.children.push(new_entry);
        }
        
        // Check for opening brace
        if open_brace_re.is_match(&line) {
            level += 1;
        }
        
        // Check for closing brace
        if close_brace_re.is_match(&line) {
            level = level.saturating_sub(1);
        }
    }
    
    Some(entry)
}

pub fn get_entry<'a>(root: &'a Entry, path: &[usize]) -> &'a Entry {
    let mut e = root;
    for &idx in path {
        e = &e.children[idx];
    }
    e
}

pub fn try_get_entry<'a>(root: &'a Entry, path: &[usize]) -> Option<&'a Entry> {
    let mut e = root;
    for &idx in path {
        if idx >= e.children.len() {
            return None;
        }
        e = &e.children[idx];
    }
    Some(e)
}

