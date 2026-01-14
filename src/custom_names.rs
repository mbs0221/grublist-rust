use std::fs;
use std::io;
use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

const CUSTOM_NAMES_FILE: &str = "/etc/grublist-custom-names.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomNames {
    pub names: HashMap<String, String>, // path -> custom_name
}

impl CustomNames {
    pub fn load() -> Self {
        if let Ok(content) = fs::read_to_string(CUSTOM_NAMES_FILE) {
            if let Ok(names) = serde_json::from_str::<CustomNames>(&content) {
                return names;
            }
        }
        CustomNames {
            names: HashMap::new(),
        }
    }
    
    pub fn save(&self) -> io::Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(CUSTOM_NAMES_FILE, content)?;
        Ok(())
    }
    
    pub fn get_custom_name(&self, path: &[usize]) -> Option<&String> {
        let path_str = path_to_string(path);
        self.names.get(&path_str)
    }
    
    pub fn set_custom_name(&mut self, path: &[usize], name: String) {
        let path_str = path_to_string(path);
        if name.is_empty() {
            self.names.remove(&path_str);
        } else {
            self.names.insert(path_str, name);
        }
    }
}

fn path_to_string(path: &[usize]) -> String {
    path.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(">")
}

pub fn string_to_path(s: &str) -> Vec<usize> {
    s.split('>')
        .filter_map(|x| x.parse::<usize>().ok())
        .collect()
}

