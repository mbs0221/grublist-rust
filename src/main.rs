mod colorprint;
mod grub;
mod grub_config;
mod kernel_info;
mod kernel_cleanup;
mod custom_names;
mod backup_manager;
mod grub_validate;
mod boot_time;

use grub::{Entry, EntryType, load_grub, get_entry, try_get_entry};
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io::{self, stdout};

fn main() -> io::Result<()> {
    let entry = match load_grub() {
        Some(e) => e,
        None => {
            eprintln!("LoadGrub Failed. \"/boot/grub/grub.cfg\" Not Found.");
            return Ok(());
        }
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(entry);
    let result = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

struct App {
    entry: Entry,
    state: AppState,
    state_stack: Vec<AppState>,
    search_query: String,
    search_results: Vec<Vec<usize>>,
    search_selected: usize,
    bcolors: colorprint::Bcolors,
}

#[derive(Clone)]
enum AppState {
    MainMenu {
        selected: usize,
    },
    SearchMode {
        selected: usize,
    },
    SelectBootEntry {
        path: Vec<usize>,
        selected: usize,
        action: Option<SelectBootEntryAction>,
    },
    SelectBootEntrySearch {
        path: Vec<usize>,
        query: String,
        results: Vec<Vec<usize>>,
        selected: usize,
    },
    ConfigureGrub {
        selected: usize,
        linux_params: Vec<String>,
        linux_default_params: Vec<String>,
        timeout: String,
        timeout_style: String,
        input_mode: GrubConfigInputMode,
        input_buffer: String,
    },
    EditParameterList {
        title: String,
        params: Vec<String>,
        selected: usize,
        input_mode: InputMode,
        input_buffer: String,
    },
    ViewDefaultEntry,
    ConfirmSetDefaultEntry {
        path: Vec<usize>,
        entry_name: String,
    },
    Message {
        title: String,
        content: Vec<String>,
        message_type: MessageType,
    },
    ViewKernelInfo {
        path: Vec<usize>,
        kernel_info: Option<kernel_info::KernelInfo>,
    },
    CleanupKernels {
        kernels: Vec<kernel_cleanup::KernelToClean>,
        selected: usize,
    },
    RenameBootEntry {
        path: Vec<usize>,
        original_name: String,
        input_buffer: String,
    },
    BackupManager {
        backups: Vec<backup_manager::BackupInfo>,
        selected: usize,
    },
    ValidateGrub {
        result: Option<grub_validate::ValidationResult>,
    },
    BootTimeStats {
        entries: Vec<boot_time::BootTimeEntry>,
        selected: usize,
    },
    EditAllGrubParams {
        params: Vec<(String, String)>,
        selected: usize,
        input_mode: GrubConfigInputMode,
        input_buffer: String,
    },
}

#[derive(Clone)]
enum MessageType {
    Success,
    Error,
    Info,
}

#[derive(PartialEq, Clone)]
enum SelectBootEntryAction {
    ViewKernelInfo,
    Rename,
}

#[derive(PartialEq, Clone)]
enum GrubConfigInputMode {
    None,
    EditTimeout,
    SelectTimeoutStyle,
    EditLinuxParams,
    EditLinuxDefaultParams,
}

#[derive(PartialEq)]
#[derive(Clone)]
enum InputMode {
    None,
    EditValue(usize),
    AddName,
    AddValue(String),
    DeleteIndex,
}

impl App {
    fn new(entry: Entry) -> Self {
        App {
            entry,
            state: AppState::MainMenu { selected: 0 },
            state_stack: Vec::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            bcolors: colorprint::Bcolors::new(),
        }
    }

    fn push_state(&mut self) {
        // Push current state to stack (clone it)
        self.state_stack.push(self.state.clone());
    }

    fn pop_state(&mut self) -> Option<AppState> {
        self.state_stack.pop()
    }

    fn navigate_to(&mut self, new_state: AppState, push_current: bool) {
        if push_current {
            self.push_state();
        }
        self.state = new_state;
    }

    fn navigate_back(&mut self) -> bool {
        if let Some(prev_state) = self.pop_state() {
            self.state = prev_state;
            true
        } else {
            // If stack is empty, go to main menu
            self.state = AppState::MainMenu { selected: 0 };
            false
        }
    }

    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Extract state to avoid borrowing conflicts
                let state_snapshot = match &self.state {
                    AppState::MainMenu { selected } => (0, *selected),
                    AppState::SearchMode { selected } => (1, *selected),
                    AppState::SelectBootEntry { path, selected, .. } => (2, *selected),
                    AppState::SelectBootEntrySearch { path: _, query: _, results: _, selected } => (3, *selected),
                    AppState::ConfigureGrub { selected, .. } => (4, *selected),
                    AppState::EditParameterList { selected, .. } => (5, *selected),
                    AppState::ViewDefaultEntry => (6, 0),
                    AppState::ConfirmSetDefaultEntry { .. } => (8, 0),
                    AppState::Message { .. } => (9, 0),
                    AppState::ViewKernelInfo { .. } => (10, 0),
                    AppState::CleanupKernels { selected, .. } => (11, *selected),
                    AppState::RenameBootEntry { .. } => (12, 0),
                    AppState::BackupManager { selected, .. } => (13, *selected),
                    AppState::ValidateGrub { .. } => (14, 0),
                    AppState::BootTimeStats { selected, .. } => (15, *selected),
                    AppState::EditAllGrubParams { selected, .. } => (16, *selected),
                };

                match state_snapshot.0 {
                    0 => { // MainMenu
                        match key.code {
                            KeyCode::Esc => break,
                            KeyCode::Up => {
                                if let AppState::MainMenu { selected } = &mut self.state {
                                    if *selected == 0 {
                                        *selected = 5;
                                    } else {
                                        *selected -= 1;
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::MainMenu { selected } = &mut self.state {
                                    *selected = (*selected + 1) % 6;
                                }
                            }
                            KeyCode::Enter | KeyCode::Right => {
                                let selected_idx = state_snapshot.1;
                                self.handle_main_menu_action(selected_idx)?;
                            }
                            _ => {}
                        }
                    }
                    1 => { // SearchMode
                        match key.code {
                            KeyCode::Esc => {
                                self.state = AppState::MainMenu { selected: 0 };
                                self.search_query.clear();
                                self.search_results.clear();
                            }
                            KeyCode::Up => {
                                if let AppState::SearchMode { selected } = &mut self.state {
                                    if !self.search_results.is_empty() {
                                        if *selected == 0 {
                                            *selected = self.search_results.len() - 1;
                                        } else {
                                            *selected -= 1;
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::SearchMode { selected } = &mut self.state {
                                    if !self.search_results.is_empty() {
                                        *selected = (*selected + 1) % self.search_results.len();
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if !self.search_results.is_empty() && state_snapshot.1 < self.search_results.len() {
                                    self.state = AppState::MainMenu { selected: 0 };
                                    self.search_query.clear();
                                    self.search_results.clear();
                                }
                            }
                            KeyCode::Backspace => {
                                self.search_query.pop();
                                self.update_search_results();
                                if let AppState::SearchMode { selected } = &mut self.state {
                                    *selected = 0;
                                }
                            }
                            _ => {
                                if let Some(c) = Self::key_to_char(&key) {
                                    self.search_query.push(c);
                                    self.update_search_results();
                                    if let AppState::SearchMode { selected } = &mut self.state {
                                        *selected = 0;
                                    }
                                }
                            }
                        }
                    }
                    2 => { // SelectBootEntry
                        match key.code {
                            KeyCode::Esc => {
                                self.navigate_back();
                            }
                            KeyCode::Up => {
                                if let AppState::SelectBootEntry { path, selected, .. } = &mut self.state {
                                    let entry_ref = if path.is_empty() {
                                        &self.entry
                                    } else {
                                        get_entry(&self.entry, path)
                                    };
                                    let len = entry_ref.children.len();
                                    if len > 0 {
                                        if *selected == 0 {
                                            *selected = len - 1;
                                        } else {
                                            *selected -= 1;
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::SelectBootEntry { path, selected, .. } = &mut self.state {
                                    let entry_ref = if path.is_empty() {
                                        &self.entry
                                    } else {
                                        get_entry(&self.entry, path)
                                    };
                                    let len = entry_ref.children.len();
                                    if len > 0 {
                                        *selected = (*selected + 1) % len;
                                    }
                                }
                            }
                            KeyCode::Enter | KeyCode::Right => {
                                // Enter/Right: navigate into submenu only
                                if let AppState::SelectBootEntry { path, selected, .. } = &self.state {
                                    let entry_ref = if path.is_empty() {
                                        &self.entry
                                    } else {
                                        get_entry(&self.entry, path)
                                    };
                                    if state_snapshot.1 < entry_ref.children.len() {
                                        let child = &entry_ref.children[state_snapshot.1];
                                        if child.entry_type == EntryType::Submenu {
                                            if let AppState::SelectBootEntry { path: p, selected: s, .. } = &mut self.state {
                                                p.push(state_snapshot.1);
                                                *s = 0;
                                            }
                                        }
                                        // For MenuEntry, use i/y/e keys instead
                                    }
                                }
                            }
                            KeyCode::Left => {
                                if let AppState::SelectBootEntry { path, selected, .. } = &mut self.state {
                                    if !path.is_empty() {
                                        // Go back to parent submenu
                                        path.pop();
                                        *selected = 0;
                                    } else {
                                        // If at root level, go back to previous state using stack
                                        self.navigate_back();
                                    }
                                }
                            }
                            KeyCode::Char('i') | KeyCode::Char('I') => {
                                // View kernel info
                                if let AppState::SelectBootEntry { path, selected, .. } = &self.state {
                                    let entry_ref = if path.is_empty() {
                                        &self.entry
                                    } else {
                                        get_entry(&self.entry, path)
                                    };
                                    if state_snapshot.1 < entry_ref.children.len() {
                                        let child = &entry_ref.children[state_snapshot.1];
                                        if child.entry_type == EntryType::MenuEntry {
                                            let mut result_path = path.clone();
                                            result_path.push(state_snapshot.1);
                                            let entry_name = child.name.clone();
                                            let kernel_info = kernel_info::get_kernel_version_from_entry(&entry_name);
                                            self.navigate_to(AppState::ViewKernelInfo {
                                                path: result_path,
                                                kernel_info,
                                            }, true);
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                // Confirm set default boot entry
                                if let AppState::SelectBootEntry { path, selected, action } = &self.state {
                                    let entry_ref = if path.is_empty() {
                                        &self.entry
                                    } else {
                                        get_entry(&self.entry, path)
                                    };
                                    if state_snapshot.1 < entry_ref.children.len() {
                                        let child = &entry_ref.children[state_snapshot.1];
                                        if child.entry_type == EntryType::MenuEntry {
                                            let mut result_path = path.clone();
                                            result_path.push(state_snapshot.1);
                                            let entry_name = child.name.clone();
                                            
                                            // Only allow if action is None (set default mode)
                                            if action.is_none() {
                                                self.navigate_to(AppState::ConfirmSetDefaultEntry {
                                                    path: result_path,
                                                    entry_name,
                                                }, true);
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('e') | KeyCode::Char('E') => {
                                // Edit boot entry name
                                if let AppState::SelectBootEntry { path, selected, action } = &self.state {
                                    let entry_ref = if path.is_empty() {
                                        &self.entry
                                    } else {
                                        get_entry(&self.entry, path)
                                    };
                                    if state_snapshot.1 < entry_ref.children.len() {
                                        let child = &entry_ref.children[state_snapshot.1];
                                        if child.entry_type == EntryType::MenuEntry {
                                            let mut result_path = path.clone();
                                            result_path.push(state_snapshot.1);
                                            let entry_name = child.name.clone();
                                            
                                            // Only allow if action is None (set default mode)
                                            if action.is_none() {
                                                let custom_names = custom_names::CustomNames::load();
                                                let current_name = custom_names.get_custom_name(&result_path)
                                                    .cloned()
                                                    .unwrap_or_else(|| entry_name.clone());
                                                self.navigate_to(AppState::RenameBootEntry {
                                                    path: result_path,
                                                    original_name: entry_name,
                                                    input_buffer: current_name,
                                                }, true);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {
                                if let Some(c) = Self::key_to_char(&key) {
                                    if let AppState::SelectBootEntry { path, action, .. } = &self.state {
                                        // Don't start search for i, y, e keys
                                        if c != 'i' && c != 'I' && c != 'y' && c != 'Y' && c != 'e' && c != 'E' {
                                            self.start_boot_entry_search(c, path.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    3 => { // SelectBootEntrySearch
                        match key.code {
                            KeyCode::Esc => {
                                self.navigate_back();
                            }
                            KeyCode::Up => {
                                if let AppState::SelectBootEntrySearch { results, selected, .. } = &mut self.state {
                                    if !results.is_empty() {
                                        if *selected == 0 {
                                            *selected = results.len() - 1;
                                        } else {
                                            *selected -= 1;
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::SelectBootEntrySearch { results, selected, .. } = &mut self.state {
                                    if !results.is_empty() {
                                        *selected = (*selected + 1) % results.len();
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if let AppState::SelectBootEntrySearch { results, .. } = &self.state {
                                    if !results.is_empty() && state_snapshot.1 < results.len() {
                                        let result_path = results[state_snapshot.1].clone();
                                        let entry_ref = get_entry(&self.entry, &result_path);
                                        let entry_name = entry_ref.name.clone();
                                        self.navigate_to(AppState::ConfirmSetDefaultEntry {
                                            path: result_path,
                                            entry_name,
                                        }, true);
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                let mut query_clone = String::new();
                                if let AppState::SelectBootEntrySearch { query, .. } = &self.state {
                                    query_clone = query.clone();
                                }
                                if !query_clone.is_empty() {
                                    query_clone.pop();
                                }
                                let new_results = self.collect_all_matches(&query_clone);
                                if let AppState::SelectBootEntrySearch { query, results, selected, .. } = &mut self.state {
                                    *query = query_clone;
                                    *results = new_results;
                                    *selected = 0;
                                }
                            }
                            _ => {
                                if let Some(c) = Self::key_to_char(&key) {
                                    let mut query_clone = String::new();
                                    if let AppState::SelectBootEntrySearch { query, .. } = &self.state {
                                        query_clone = query.clone();
                                    }
                                    query_clone.push(c);
                                    let new_results = self.collect_all_matches(&query_clone);
                                    if let AppState::SelectBootEntrySearch { query, results, selected, .. } = &mut self.state {
                                        *query = query_clone;
                                        *results = new_results;
                                        *selected = 0;
                                    }
                                }
                            }
                        }
                    }
                    4 => { // ConfigureGrub
                        match key.code {
                            KeyCode::Esc | KeyCode::Left => {
                                self.navigate_back();
                            }
                            KeyCode::Up => {
                                if let AppState::ConfigureGrub { selected, input_mode, .. } = &mut self.state {
                                    if *input_mode == GrubConfigInputMode::None {
                                        if *selected == 0 {
                                            *selected = 7;
                                        } else {
                                            *selected -= 1;
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::ConfigureGrub { selected, input_mode, .. } = &mut self.state {
                                    if *input_mode == GrubConfigInputMode::None {
                                        *selected = (*selected + 1) % 8;
                                    }
                                }
                            }
                            KeyCode::Enter | KeyCode::Right => {
                                let is_right_key = matches!(key.code, KeyCode::Right);
                                if let AppState::ConfigureGrub { selected, linux_params, linux_default_params, timeout, timeout_style, input_mode, input_buffer } = &mut self.state {
                                    let current_input_mode = input_mode.clone();
                                    match current_input_mode {
                                        GrubConfigInputMode::None => {
                                            match *selected {
                                                0 => {
                                                    // Edit GRUB_CMDLINE_LINUX
                                                    let params = linux_params.clone();
                                                    self.navigate_to(AppState::EditParameterList {
                                                        title: "Edit GRUB_CMDLINE_LINUX".to_string(),
                                                        params,
                                                        selected: 0,
                                                        input_mode: InputMode::None,
                                                        input_buffer: String::new(),
                                                    }, true);
                                                }
                                                1 => {
                                                    // Edit GRUB_CMDLINE_LINUX_DEFAULT
                                                    let params = linux_default_params.clone();
                                                    self.navigate_to(AppState::EditParameterList {
                                                        title: "Edit GRUB_CMDLINE_LINUX_DEFAULT".to_string(),
                                                        params,
                                                        selected: 0,
                                                        input_mode: InputMode::None,
                                                        input_buffer: String::new(),
                                                    }, true);
                                                }
                                                2 => {
                                                    // Edit timeout
                                                    *input_mode = GrubConfigInputMode::EditTimeout;
                                                    *input_buffer = timeout.clone();
                                                }
                                                3 => {
                                                    // Edit timeout style
                                                    *input_mode = GrubConfigInputMode::SelectTimeoutStyle;
                                                    *input_buffer = String::new();
                                                }
                                                4 => {
                                                    // View/Edit All Parameters
                                                    match grub_config::GrubConfig::load() {
                                                        Ok(config) => {
                                                            let mut params: Vec<(String, String)> = config.get_all_params()
                                                                .iter()
                                                                .map(|(k, v)| (k.clone(), v.clone()))
                                                                .collect();
                                                            params.sort_by(|a, b| a.0.cmp(&b.0));
                                                            self.navigate_to(AppState::EditAllGrubParams {
                                                                params,
                                                                selected: 0,
                                                                input_mode: GrubConfigInputMode::None,
                                                                input_buffer: String::new(),
                                                            }, true);
                                                        }
                                                        Err(e) => {
                                                            self.state = AppState::Message {
                                                                title: "Error".to_string(),
                                                                content: vec![format!("Error loading config: {}", e)],
                                                                message_type: MessageType::Error,
                                                            };
                                                        }
                                                    }
                                                }
                                                5 => {
                                                    // Validate GRUB Config
                                                    match grub_validate::validate_grub_config() {
                                                        Ok(result) => {
                                                            self.navigate_to(AppState::ValidateGrub {
                                                                result: Some(result),
                                                            }, true);
                                                        }
                                                        Err(e) => {
                                                            self.state = AppState::Message {
                                                                title: "Error".to_string(),
                                                                content: vec![format!("Failed to validate GRUB config: {}", e)],
                                                                message_type: MessageType::Error,
                                                            };
                                                        }
                                                    }
                                                }
                                                6 => {
                                                    // Save
                                                    let mut config = match grub_config::GrubConfig::load() {
                                                        Ok(c) => c,
                                                        Err(e) => {
                                                            self.state = AppState::Message {
                                                                title: "Error".to_string(),
                                                                content: vec![format!("Error loading config: {}", e)],
                                                                message_type: MessageType::Error,
                                                            };
                                                            continue;
                                                        }
                                                    };
                                                    config.grub_cmdline_linux = grub_config::join_parameters(linux_params);
                                                    config.grub_cmdline_linux_default = grub_config::join_parameters(linux_default_params);
                                                    config.grub_timeout = timeout.clone();
                                                    config.grub_timeout_style = timeout_style.clone();
                                                    
                                                    match config.save() {
                                                        Ok(_) => {
                                                            self.state = AppState::Message {
                                                                title: "Success".to_string(),
                                                                content: vec![
                                                                    "Configuration saved successfully!".to_string(),
                                                                    "".to_string(),
                                                                    "Please run:".to_string(),
                                                                    "  sudo update-grub".to_string(),
                                                                ],
                                                                message_type: MessageType::Success,
                                                            };
                                                        }
                                                        Err(e) => {
                                                            self.state = AppState::Message {
                                                                title: "Error".to_string(),
                                                                content: vec![format!("Error saving configuration: {}", e)],
                                                                message_type: MessageType::Error,
                                                            };
                                                        }
                                                    }
                                                }
                                                7 => {
                                                    // Cancel
                                                    self.navigate_back();
                                                }
                                                _ => {}
                                            }
                                        }
                                        GrubConfigInputMode::EditTimeout => {
                                            if !input_buffer.trim().is_empty() {
                                                *timeout = input_buffer.trim().to_string();
                                            }
                                            
                                            if is_right_key {
                                                *selected = 3;
                                                *input_mode = GrubConfigInputMode::SelectTimeoutStyle;
                                                *input_buffer = String::new();
                                            } else {
                                                *input_mode = GrubConfigInputMode::None;
                                                *input_buffer = String::new();
                                            }
                                        }
                                        GrubConfigInputMode::SelectTimeoutStyle => {
                                            let style = input_buffer.trim().to_lowercase();
                                            if style == "menu" || style == "hidden" || style == "countdown" {
                                                *timeout_style = style;
                                            }
                                            *input_mode = GrubConfigInputMode::None;
                                            *input_buffer = String::new();
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                if let AppState::ConfigureGrub { input_mode, input_buffer, .. } = &mut self.state {
                                    if *input_mode != GrubConfigInputMode::None {
                                        input_buffer.pop();
                                    }
                                }
                            }
                            _ => {
                                if let AppState::ConfigureGrub { input_mode, input_buffer, .. } = &mut self.state {
                                    if *input_mode != GrubConfigInputMode::None {
                                        if let Some(c) = Self::key_to_char(&key) {
                                            input_buffer.push(c);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    5 => { // EditParameterList
                        match key.code {
                            KeyCode::Esc | KeyCode::Left => {
                                if let AppState::EditParameterList { title, params, input_mode, .. } = &self.state {
                                    // If in input mode, exit input mode first
                                    if *input_mode != InputMode::None {
                                        if let AppState::EditParameterList { input_mode, input_buffer, .. } = &mut self.state {
                                            *input_mode = InputMode::None;
                                            *input_buffer = String::new();
                                        }
                                    } else {
                                        // Return to GRUB config menu with updated params
                                        let params = params.clone();
                                        let is_linux = title == "Edit GRUB_CMDLINE_LINUX";
                                        
                                        match grub_config::GrubConfig::load() {
                                            Ok(mut config) => {
                                                let linux_params = grub_config::parse_parameters(&config.grub_cmdline_linux);
                                                let linux_default_params = grub_config::parse_parameters(&config.grub_cmdline_linux_default);
                                                
                                                if is_linux {
                                                    self.state = AppState::ConfigureGrub {
                                                        selected: 0,
                                                        linux_params: params,
                                                        linux_default_params,
                                                        timeout: config.grub_timeout,
                                                        timeout_style: config.grub_timeout_style,
                                                        input_mode: GrubConfigInputMode::None,
                                                        input_buffer: String::new(),
                                                    };
                                                } else {
                                                    self.state = AppState::ConfigureGrub {
                                                        selected: 1,
                                                        linux_params,
                                                        linux_default_params: params,
                                                        timeout: config.grub_timeout,
                                                        timeout_style: config.grub_timeout_style,
                                                        input_mode: GrubConfigInputMode::None,
                                                        input_buffer: String::new(),
                                                    };
                                                }
                                            }
                                            Err(_) => {
                                                self.navigate_back();
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if let AppState::EditParameterList { params, selected, input_mode, .. } = &mut self.state {
                                    if *input_mode == InputMode::None {
                                        // Calculate max index: params + add + (delete if params not empty) + save + cancel
                                        let max_idx = if params.is_empty() {
                                            params.len() + 2 // add + save + cancel
                                        } else {
                                            params.len() + 3 // add + delete + save + cancel
                                        };
                                        if max_idx > 0 {
                                            if *selected == 0 {
                                                *selected = max_idx;
                                            } else {
                                                *selected -= 1;
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::EditParameterList { params, selected, input_mode, .. } = &mut self.state {
                                    if *input_mode == InputMode::None {
                                        // Calculate max index: params + add + (delete if params not empty) + save + cancel
                                        let max_idx = if params.is_empty() {
                                            params.len() + 2 // add + save + cancel
                                        } else {
                                            params.len() + 3 // add + delete + save + cancel
                                        };
                                        if max_idx > 0 {
                                            *selected = (*selected + 1) % (max_idx + 1);
                                        }
                                    }
                                }
                            }
                            KeyCode::Enter | KeyCode::Right => {
                                let is_right_key = matches!(key.code, KeyCode::Right);
                                if let AppState::EditParameterList { params, selected, input_mode, input_buffer, .. } = &mut self.state {
                                    let current_input_mode = input_mode.clone();
                                    match current_input_mode {
                                        InputMode::None => {
                                            if *selected < params.len() {
                                                // Edit parameter
                                                *input_mode = InputMode::EditValue(*selected);
                                                *input_buffer = String::new();
                                            } else if *selected == params.len() {
                                                // Add parameter
                                                *input_mode = InputMode::AddName;
                                                *input_buffer = String::new();
                                            } else if *selected == params.len() + 1 {
                                                if !params.is_empty() {
                                                    // Delete parameter
                                                    *input_mode = InputMode::DeleteIndex;
                                                    *input_buffer = String::new();
                                                } else {
                                                    // Save (when params is empty, delete option is not shown)
                                                    // Return to kernel params menu
                                                    if let AppState::EditParameterList { title, params, .. } = &self.state {
                                                        let params = params.clone();
                                                        let is_linux = title == "Edit GRUB_CMDLINE_LINUX";
                                                        
                                                        match grub_config::GrubConfig::load() {
                                                            Ok(mut config) => {
                                                                let linux_params = grub_config::parse_parameters(&config.grub_cmdline_linux);
                                                                let linux_default_params = grub_config::parse_parameters(&config.grub_cmdline_linux_default);
                                                                
                                                                if is_linux {
                                                                    self.state = AppState::ConfigureGrub {
                                                                        selected: 0,
                                                                        linux_params: params,
                                                                        linux_default_params,
                                                                        timeout: config.grub_timeout,
                                                                        timeout_style: config.grub_timeout_style,
                                                                        input_mode: GrubConfigInputMode::None,
                                                                        input_buffer: String::new(),
                                                                    };
                                                                } else {
                                                                    self.state = AppState::ConfigureGrub {
                                                                        selected: 1,
                                                                        linux_params,
                                                                        linux_default_params: params,
                                                                        timeout: config.grub_timeout,
                                                                        timeout_style: config.grub_timeout_style,
                                                                        input_mode: GrubConfigInputMode::None,
                                                                        input_buffer: String::new(),
                                                                    };
                                                                }
                                                            }
                                                            Err(_) => {
                                                                self.state = AppState::MainMenu { selected: 0 };
                                                            }
                                                        }
                                                    }
                                                }
                                            } else if *selected == params.len() + 2 {
                                                if params.is_empty() {
                                                    // Cancel (when params is empty, save is at index len+1, cancel is at len+2)
                                                    if let AppState::EditParameterList { title, .. } = &self.state {
                                                        let is_linux = title == "Edit GRUB_CMDLINE_LINUX";
                                                        
                                                        match grub_config::GrubConfig::load() {
                                                            Ok(config) => {
                                                                let linux_params = grub_config::parse_parameters(&config.grub_cmdline_linux);
                                                                let linux_default_params = grub_config::parse_parameters(&config.grub_cmdline_linux_default);
                                                                
                                                                self.state = AppState::ConfigureGrub {
                                                                    selected: if is_linux { 0 } else { 1 },
                                                                    linux_params,
                                                                    linux_default_params,
                                                                    timeout: config.grub_timeout,
                                                                    timeout_style: config.grub_timeout_style,
                                                                    input_mode: GrubConfigInputMode::None,
                                                                    input_buffer: String::new(),
                                                                };
                                                            }
                                                            Err(_) => {
                                                                self.state = AppState::MainMenu { selected: 0 };
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    // Save (when params is not empty)
                                                    // Return to kernel params menu
                                                    if let AppState::EditParameterList { title, params, .. } = &self.state {
                                                        let params = params.clone();
                                                        let is_linux = title == "Edit GRUB_CMDLINE_LINUX";
                                                        
                                                        match grub_config::GrubConfig::load() {
                                                            Ok(mut config) => {
                                                                let linux_params = grub_config::parse_parameters(&config.grub_cmdline_linux);
                                                                let linux_default_params = grub_config::parse_parameters(&config.grub_cmdline_linux_default);
                                                                
                                                                if is_linux {
                                                                    self.state = AppState::ConfigureGrub {
                                                                        selected: 0,
                                                                        linux_params: params,
                                                                        linux_default_params,
                                                                        timeout: config.grub_timeout,
                                                                        timeout_style: config.grub_timeout_style,
                                                                        input_mode: GrubConfigInputMode::None,
                                                                        input_buffer: String::new(),
                                                                    };
                                                                } else {
                                                                    self.state = AppState::ConfigureGrub {
                                                                        selected: 1,
                                                                        linux_params,
                                                                        linux_default_params: params,
                                                                        timeout: config.grub_timeout,
                                                                        timeout_style: config.grub_timeout_style,
                                                                        input_mode: GrubConfigInputMode::None,
                                                                        input_buffer: String::new(),
                                                                    };
                                                                }
                                                            }
                                                            Err(_) => {
                                                                self.state = AppState::MainMenu { selected: 0 };
                                                            }
                                                        }
                                                    }
                                                }
                                            } else if *selected == params.len() + 3 {
                                                // Cancel (when params is not empty)
                                                if let AppState::EditParameterList { title, .. } = &self.state {
                                                    let is_linux = title == "Edit GRUB_CMDLINE_LINUX";
                                                    
                                                    match grub_config::GrubConfig::load() {
                                                        Ok(config) => {
                                                            let linux_params = grub_config::parse_parameters(&config.grub_cmdline_linux);
                                                            let linux_default_params = grub_config::parse_parameters(&config.grub_cmdline_linux_default);
                                                            
                                                            self.state = AppState::ConfigureGrub {
                                                                selected: if is_linux { 0 } else { 1 },
                                                                linux_params,
                                                                linux_default_params,
                                                                timeout: config.grub_timeout,
                                                                timeout_style: config.grub_timeout_style,
                                                                input_mode: GrubConfigInputMode::None,
                                                                input_buffer: String::new(),
                                                            };
                                                        }
                                                        Err(_) => {
                                                            self.state = AppState::MainMenu { selected: 0 };
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        InputMode::EditValue(idx) => {
                                            if !input_buffer.is_empty() {
                                                let (name, _) = grub_config::split_parameter(&params[idx]);
                                                params[idx] = grub_config::format_parameter(&name, Some(input_buffer.trim()));
                                            }
                                            
                                            // If Right key was pressed, move to next parameter and enter edit mode
                                            if is_right_key {
                                                let next_idx = idx + 1;
                                                if next_idx < params.len() {
                                                    *selected = next_idx;
                                                    *input_mode = InputMode::EditValue(next_idx);
                                                    *input_buffer = String::new();
                                                } else {
                                                    // No more parameters, exit edit mode
                                                    *input_mode = InputMode::None;
                                                    *input_buffer = String::new();
                                                }
                                            } else {
                                                // Enter key, just exit edit mode
                                                *input_mode = InputMode::None;
                                                *input_buffer = String::new();
                                            }
                                        }
                                        InputMode::AddName => {
                                            if !input_buffer.trim().is_empty() {
                                                *input_mode = InputMode::AddValue(input_buffer.trim().to_string());
                                                *input_buffer = String::new();
                                            } else {
                                                *input_mode = InputMode::None;
                                                *input_buffer = String::new();
                                            }
                                        }
                                        InputMode::AddValue(name) => {
                                            let param = if input_buffer.trim().is_empty() {
                                                name.clone()
                                            } else {
                                                grub_config::format_parameter(&name, Some(input_buffer.trim()))
                                            };
                                            params.push(param);
                                            *input_mode = InputMode::None;
                                            *input_buffer = String::new();
                                        }
                                        InputMode::DeleteIndex => {
                                            if let Ok(idx) = input_buffer.trim().parse::<usize>() {
                                                if idx >= 1 && idx <= params.len() {
                                                    params.remove(idx - 1);
                                                }
                                            }
                                            *input_mode = InputMode::None;
                                            *input_buffer = String::new();
                                        }
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                if let AppState::EditParameterList { input_mode, input_buffer, .. } = &mut self.state {
                                    if !matches!(input_mode, InputMode::None) {
                                        input_buffer.pop();
                                    }
                                }
                            }
                            _ => {
                                if let Some(c) = Self::key_to_char(&key) {
                                    if let AppState::EditParameterList { input_mode, input_buffer, .. } = &mut self.state {
                                        if !matches!(input_mode, InputMode::None) {
                                            input_buffer.push(c);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    6 => { // ViewDefaultEntry
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter | KeyCode::Left => {
                                self.navigate_back();
                            }
                            _ => {}
                        }
                    }
                    8 => { // ConfirmSetDefaultEntry
                        match key.code {
                            KeyCode::Esc => {
                                self.navigate_back();
                            }
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                if let AppState::ConfirmSetDefaultEntry { path, .. } = &self.state {
                                    let p_str: String = path.iter()
                                        .map(|x| x.to_string())
                                        .collect::<Vec<_>>()
                                        .join(">");
                                    
                                    match grub_config::GrubConfig::load() {
                                        Ok(mut config) => {
                                            config.grub_default = format!("\"{}\"", p_str);
                                            
                                            match config.save() {
                                                Ok(_) => {
                                                    self.state = AppState::Message {
                                                        title: "Success".to_string(),
                                                        content: vec![
                                                            "Default boot entry set successfully!".to_string(),
                                                            "".to_string(),
                                                            "Please run:".to_string(),
                                                            "  sudo update-grub".to_string(),
                                                        ],
                                                        message_type: MessageType::Success,
                                                    };
                                                }
                                                Err(e) => {
                                                    self.state = AppState::Message {
                                                        title: "Error".to_string(),
                                                        content: vec![
                                                            format!("Error saving configuration: {}", e),
                                                        ],
                                                        message_type: MessageType::Error,
                                                    };
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            self.state = AppState::Message {
                                                title: "Error".to_string(),
                                                content: vec![
                                                    format!("Error loading config: {}", e),
                                                ],
                                                message_type: MessageType::Error,
                                            };
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                self.navigate_back();
                            }
                            _ => {}
                        }
                    }
                    9 => { // Message
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                self.state = AppState::MainMenu { selected: 0 };
                            }
                            _ => {}
                        }
                    }
                    10 => { // ViewKernelInfo
                        match key.code {
                            KeyCode::Esc | KeyCode::Left => {
                                self.navigate_back();
                            }
                            _ => {}
                        }
                    }
                    11 => { // CleanupKernels
                        match key.code {
                            KeyCode::Esc | KeyCode::Left => {
                                self.navigate_back();
                            }
                            KeyCode::Up => {
                                if let AppState::CleanupKernels { kernels, selected } = &mut self.state {
                                    if kernels.is_empty() {
                                        return Ok(());
                                    }
                                    if *selected == 0 {
                                        *selected = kernels.len() - 1;
                                    } else {
                                        *selected -= 1;
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::CleanupKernels { kernels, selected } = &mut self.state {
                                    if kernels.is_empty() {
                                        return Ok(());
                                    }
                                    *selected = (*selected + 1) % kernels.len();
                                }
                            }
                            KeyCode::Enter => {
                                if let AppState::CleanupKernels { kernels, selected } = &self.state {
                                    if let Some(kernel) = kernels.get(*selected) {
                                        if !kernel.in_use {
                                            match kernel_cleanup::delete_kernel_files(&kernel.version) {
                                                Ok(_) => {
                                                    let mut new_kernels = kernel_cleanup::scan_unused_kernels();
                                                    let new_selected = (*selected).min(new_kernels.len().saturating_sub(1));
                                                    self.state = AppState::CleanupKernels {
                                                        kernels: new_kernels,
                                                        selected: new_selected,
                                                    };
                                                }
                                                Err(e) => {
                                                    self.state = AppState::Message {
                                                        title: "Error".to_string(),
                                                        content: vec![format!("Failed to delete kernel: {}", e)],
                                                        message_type: MessageType::Error,
                                                    };
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    12 => { // RenameBootEntry
                        match key.code {
                            KeyCode::Esc | KeyCode::Left => {
                                self.navigate_back();
                            }
                            KeyCode::Enter => {
                                if let AppState::RenameBootEntry { path, input_buffer, .. } = &self.state {
                                    let mut custom_names = custom_names::CustomNames::load();
                                    custom_names.set_custom_name(path, input_buffer.clone());
                                    if let Err(e) = custom_names.save() {
                                        self.state = AppState::Message {
                                            title: "Error".to_string(),
                                            content: vec![format!("Failed to save custom name: {}", e)],
                                            message_type: MessageType::Error,
                                        };
                                    } else {
                                        self.state = AppState::Message {
                                            title: "Success".to_string(),
                                            content: vec!["Custom name saved successfully!".to_string()],
                                            message_type: MessageType::Success,
                                        };
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                if let AppState::RenameBootEntry { input_buffer, .. } = &mut self.state {
                                    input_buffer.pop();
                                }
                            }
                            _ => {
                                if let Some(c) = Self::key_to_char(&key) {
                                    if let AppState::RenameBootEntry { input_buffer, .. } = &mut self.state {
                                        input_buffer.push(c);
                                    }
                                }
                            }
                        }
                    }
                    13 => { // BackupManager
                        match key.code {
                            KeyCode::Esc | KeyCode::Left => {
                                self.navigate_back();
                            }
                            KeyCode::Up => {
                                if let AppState::BackupManager { backups, selected } = &mut self.state {
                                    if backups.is_empty() {
                                        return Ok(());
                                    }
                                    if *selected == 0 {
                                        *selected = backups.len() - 1;
                                    } else {
                                        *selected -= 1;
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::BackupManager { backups, selected } = &mut self.state {
                                    if backups.is_empty() {
                                        return Ok(());
                                    }
                                    *selected = (*selected + 1) % backups.len();
                                }
                            }
                            KeyCode::Enter => {
                                if let AppState::BackupManager { backups, selected } = &self.state {
                                    if let Some(backup) = backups.get(*selected) {
                                        match backup_manager::restore_backup(&backup.path) {
                                            Ok(_) => {
                                                self.state = AppState::Message {
                                                    title: "Success".to_string(),
                                                    content: vec![
                                                        "Backup restored successfully!".to_string(),
                                                        "".to_string(),
                                                        "Please run:".to_string(),
                                                        "  sudo update-grub".to_string(),
                                                    ],
                                                    message_type: MessageType::Success,
                                                };
                                            }
                                            Err(e) => {
                                                self.state = AppState::Message {
                                                    title: "Error".to_string(),
                                                    content: vec![format!("Failed to restore backup: {}", e)],
                                                    message_type: MessageType::Error,
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('d') | KeyCode::Char('D') => {
                                if let AppState::BackupManager { backups, selected } = &self.state {
                                    if let Some(backup) = backups.get(*selected) {
                                        match backup_manager::delete_backup(&backup.path) {
                                            Ok(_) => {
                                                let new_backups = backup_manager::list_backups();
                                                let new_selected = (*selected).min(new_backups.len().saturating_sub(1));
                                                self.state = AppState::BackupManager {
                                                    backups: new_backups,
                                                    selected: new_selected,
                                                };
                                            }
                                            Err(e) => {
                                                self.state = AppState::Message {
                                                    title: "Error".to_string(),
                                                    content: vec![format!("Failed to delete backup: {}", e)],
                                                    message_type: MessageType::Error,
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    14 => { // ValidateGrub
                        match key.code {
                            KeyCode::Esc | KeyCode::Enter | KeyCode::Left => {
                                self.navigate_back();
                            }
                            _ => {}
                        }
                    }
                    15 => { // BootTimeStats
                        match key.code {
                            KeyCode::Esc | KeyCode::Left => {
                                self.navigate_back();
                            }
                            KeyCode::Enter => {
                                // Enter does nothing in BootTimeStats, just go back
                                self.navigate_back();
                            }
                            KeyCode::Up => {
                                if let AppState::BootTimeStats { entries, selected } = &mut self.state {
                                    if !entries.is_empty() {
                                        if *selected == 0 {
                                            *selected = entries.len() - 1;
                                        } else {
                                            *selected -= 1;
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::BootTimeStats { entries, selected } = &mut self.state {
                                    if !entries.is_empty() {
                                        *selected = (*selected + 1) % entries.len();
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    16 => { // EditAllGrubParams
                        match key.code {
                            KeyCode::Esc | KeyCode::Left => {
                                self.navigate_back();
                            }
                            KeyCode::Up => {
                                if let AppState::EditAllGrubParams { params, selected, input_mode, .. } = &mut self.state {
                                    if *input_mode == GrubConfigInputMode::None {
                                        if !params.is_empty() {
                                            if *selected == 0 {
                                                *selected = params.len() - 1;
                                            } else {
                                                *selected -= 1;
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::EditAllGrubParams { params, selected, input_mode, .. } = &mut self.state {
                                    if *input_mode == GrubConfigInputMode::None {
                                        if !params.is_empty() {
                                            *selected = (*selected + 1) % params.len();
                                        }
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if let AppState::EditAllGrubParams { params, selected, input_mode, input_buffer } = &mut self.state {
                                    match input_mode {
                                        GrubConfigInputMode::None => {
                                            if !params.is_empty() && *selected < params.len() {
                                                *input_mode = GrubConfigInputMode::EditTimeout;
                                                *input_buffer = params[*selected].1.clone();
                                            }
                                        }
                                        GrubConfigInputMode::EditTimeout => {
                                            // Save the edited value
                                            if *selected < params.len() {
                                                params[*selected].1 = input_buffer.clone();
                                                *input_mode = GrubConfigInputMode::None;
                                                *input_buffer = String::new();
                                                
                                                // Save to config
                                                match grub_config::GrubConfig::load() {
                                                    Ok(mut config) => {
                                                        config.set(&params[*selected].0, input_buffer.clone());
                                                        if let Err(e) = config.save() {
                                                            self.state = AppState::Message {
                                                                title: "Error".to_string(),
                                                                content: vec![format!("Error saving parameter: {}", e)],
                                                                message_type: MessageType::Error,
                                                            };
                                                        } else {
                                                            // Reload parameters to reflect changes
                                                            match grub_config::GrubConfig::load() {
                                                                Ok(updated_config) => {
                                                                    let mut updated_params: Vec<(String, String)> = updated_config.get_all_params()
                                                                        .iter()
                                                                        .map(|(k, v)| (k.clone(), v.clone()))
                                                                        .collect();
                                                                    updated_params.sort_by(|a, b| a.0.cmp(&b.0));
                                                                    // Find the same parameter index
                                                                    let new_selected = updated_params.iter()
                                                                        .position(|(k, _)| k == &params[*selected].0)
                                                                        .unwrap_or(*selected);
                                                                    self.state = AppState::EditAllGrubParams {
                                                                        params: updated_params,
                                                                        selected: new_selected,
                                                                        input_mode: GrubConfigInputMode::None,
                                                                        input_buffer: String::new(),
                                                                    };
                                                                }
                                                                Err(e) => {
                                                                    self.state = AppState::Message {
                                                                        title: "Error".to_string(),
                                                                        content: vec![format!("Error reloading config: {}", e)],
                                                                        message_type: MessageType::Error,
                                                                    };
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        self.state = AppState::Message {
                                                            title: "Error".to_string(),
                                                            content: vec![format!("Error loading config: {}", e)],
                                                            message_type: MessageType::Error,
                                                        };
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                if let AppState::EditAllGrubParams { input_mode, input_buffer, .. } = &mut self.state {
                                    if *input_mode == GrubConfigInputMode::EditTimeout {
                                        if c == '\n' || c == '\r' {
                                            // Enter key handled above
                                        } else if c == '\x08' || c == '\x7f' {
                                            // Backspace
                                            input_buffer.pop();
                                        } else {
                                            input_buffer.push(c);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn handle_main_menu_action(&mut self, selected: usize) -> io::Result<()> {
        match selected {
            0 => {
                // Set Default Boot Entry
                self.navigate_to(AppState::SelectBootEntry {
                    path: vec![],
                    selected: 0,
                    action: None,
                }, true);
            }
            1 => {
                // View Default Boot Entry
                self.navigate_to(AppState::ViewDefaultEntry, true);
            }
            2 => {
                // Configure GRUB Settings
                match grub_config::GrubConfig::load() {
                    Ok(config) => {
                        let linux_params = grub_config::parse_parameters(&config.grub_cmdline_linux);
                        let linux_default_params = grub_config::parse_parameters(&config.grub_cmdline_linux_default);
                        self.navigate_to(AppState::ConfigureGrub {
                            selected: 0,
                            linux_params,
                            linux_default_params,
                            timeout: config.grub_timeout,
                            timeout_style: config.grub_timeout_style,
                            input_mode: GrubConfigInputMode::None,
                            input_buffer: String::new(),
                        }, true);
                    }
                    Err(e) => {
                        // TODO: Show error message
                        return Ok(());
                    }
                }
            }
            3 => {
                // Cleanup Old Kernels
                let kernels = kernel_cleanup::scan_unused_kernels();
                self.navigate_to(AppState::CleanupKernels {
                    kernels,
                    selected: 0,
                }, true);
            }
            4 => {
                // Backup Manager
                let backups = backup_manager::list_backups();
                self.navigate_to(AppState::BackupManager {
                    backups,
                    selected: 0,
                }, true);
            }
            5 => {
                // Boot Time Statistics
                let entries = boot_time::get_boot_times();
                self.navigate_to(AppState::BootTimeStats {
                    entries,
                    selected: 0,
                }, true);
            }
            _ => {}
        }
        Ok(())
    }

    fn start_search(&mut self, c: char) {
        self.search_query.clear();
        self.search_query.push(c);
        self.update_search_results();
        self.state = AppState::SearchMode { selected: 0 };
    }

    fn start_boot_entry_search(&mut self, c: char, path: Vec<usize>) {
        let mut query = String::new();
        query.push(c);
        let results = self.collect_all_matches(&query);
        self.navigate_to(AppState::SelectBootEntrySearch {
            path,
            query,
            results,
            selected: 0,
        }, true);
    }

    fn update_search_results(&mut self) {
        self.search_results = self.collect_all_matches(&self.search_query);
    }

    fn collect_all_matches(&self, query: &str) -> Vec<Vec<usize>> {
        if query.is_empty() {
            return Vec::new();
        }
        let query_lower = query.to_lowercase();
        let mut matches = Vec::new();

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
        search_recursive(&self.entry, &query_lower, &mut path, &mut matches);
        matches
    }

    fn key_to_char(key: &KeyEvent) -> Option<char> {
        match key.code {
            KeyCode::Char(c) => Some(c),
            _ => None,
        }
    }

    fn ui(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(9), // Banner
                Constraint::Min(0),    // Content
            ])
            .split(f.size());

        // Banner
        let banner = Paragraph::new(vec![
            Line::from("╔═══════════════════════════════════════════════════╗"),
            Line::from("║                                                   ║"),
            Line::from(vec![
                Span::styled("            GRUBLIST", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" v0.2.0                        "),
            ]),
            Line::from("║                                                   ║"),
            Line::from("║     Interactive GRUB Boot Menu Selector           ║"),
            Line::from("║                                                   ║"),
            Line::from("╚═══════════════════════════════════════════════════╝"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Controls: ", Style::default().fg(Color::Blue)),
                Span::raw("↑↓ Navigate  →/Enter Select  ESC Back/Quit  Type to Search"),
            ]),
        ])
        .alignment(Alignment::Center)
        .block(Block::default());
        f.render_widget(banner, chunks[0]);

        // Content
        match &self.state {
            AppState::MainMenu { selected } => {
                let items: Vec<ListItem> = vec![
                    ListItem::new("⭐ Set Default Boot Entry"),
                    ListItem::new("👁 View Default Boot Entry"),
                    ListItem::new("⚙ Configure GRUB Settings"),
                    ListItem::new("🧹 Cleanup Old Kernels"),
                    ListItem::new("💾 Backup Manager"),
                    ListItem::new("⏱ Boot Time Statistics"),
                ]
                .into_iter()
                .map(|item| item.style(Style::default().fg(Color::White)))
                .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Main Menu"))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");

                let mut state = ListState::default();
                state.select(Some(*selected));
                f.render_stateful_widget(list, chunks[1], &mut state);
            }
            AppState::SearchMode { selected } => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(0),
                    ])
                    .split(chunks[1]);

                let search_input = Paragraph::new(self.search_query.as_str())
                    .block(Block::default().borders(Borders::ALL).title("Search"))
                    .style(Style::default().fg(Color::Yellow));
                f.render_widget(search_input, chunks[0]);

                let items: Vec<ListItem> = self.search_results
                    .iter()
                    .map(|path| {
                        let entry_ref = get_entry(&self.entry, path);
                        let tag = match entry_ref.entry_type {
                            EntryType::Submenu => "[+] ",
                            EntryType::MenuEntry => "[●] ",
                            EntryType::Root => "",
                        };
                        ListItem::new(format!("{}{}", tag, entry_ref.name))
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(format!("Search Results ({} found)", self.search_results.len())))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");

                let mut state = ListState::default();
                state.select(Some(*selected));
                f.render_stateful_widget(list, chunks[1], &mut state);
            }
            AppState::SelectBootEntry { path, selected, action: _ } => {
                let entry_ref = if path.is_empty() {
                    &self.entry
                } else {
                    get_entry(&self.entry, path)
                };
                let items: Vec<ListItem> = entry_ref.children
                    .iter()
                    .map(|child| {
                        let tag = match child.entry_type {
                            EntryType::Submenu => "[+] ",
                            EntryType::MenuEntry => "[●] ",
                            EntryType::Root => "",
                        };
                        ListItem::new(format!("{}{}", tag, child.name))
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Select Boot Entry to Set as Default"))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");

                let mut state = ListState::default();
                state.select(Some(*selected));
                f.render_stateful_widget(list, chunks[1], &mut state);
            }
            AppState::SelectBootEntrySearch { path: _, query, results, selected } => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(0),
                    ])
                    .split(chunks[1]);

                let search_input = Paragraph::new(query.as_str())
                    .block(Block::default().borders(Borders::ALL).title("Search"))
                    .style(Style::default().fg(Color::Yellow));
                f.render_widget(search_input, chunks[0]);

                let items: Vec<ListItem> = results
                    .iter()
                    .map(|path| {
                        let entry_ref = get_entry(&self.entry, path);
                        let tag = match entry_ref.entry_type {
                            EntryType::Submenu => "[+] ",
                            EntryType::MenuEntry => "[●] ",
                            EntryType::Root => "",
                        };
                        ListItem::new(format!("{}{}", tag, entry_ref.name))
                    })
                    .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(format!("Search Results ({} found)", results.len())))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");

                let mut state = ListState::default();
                state.select(Some(*selected));
                f.render_stateful_widget(list, chunks[1], &mut state);
            }
            AppState::ConfigureGrub { selected, linux_params, linux_default_params, timeout, timeout_style, input_mode, input_buffer } => {
                let mut items: Vec<ListItem> = vec![
                    ListItem::new(format!("1. GRUB_CMDLINE_LINUX ({})", 
                        if linux_params.is_empty() { "empty".to_string() } 
                        else { format!("{} parameters", linux_params.len()) })),
                    ListItem::new(format!("2. GRUB_CMDLINE_LINUX_DEFAULT ({})", 
                        if linux_default_params.is_empty() { "empty".to_string() } 
                        else { format!("{} parameters", linux_default_params.len()) })),
                ];
                
                // Add timeout configuration items
                let timeout_display = match input_mode {
                    GrubConfigInputMode::EditTimeout => input_buffer.clone(),
                    _ => timeout.clone(),
                };
                items.push(ListItem::new(format!("3. GRUB_TIMEOUT: {}", timeout_display)));
                
                let timeout_style_display = match input_mode {
                    GrubConfigInputMode::SelectTimeoutStyle => input_buffer.clone(),
                    _ => timeout_style.clone(),
                };
                items.push(ListItem::new(format!("4. GRUB_TIMEOUT_STYLE: {}", timeout_style_display)));
                
                items.push(ListItem::new("5. View/Edit All Parameters"));
                items.push(ListItem::new("6. Validate GRUB Config"));
                items.push(ListItem::new("7. Save"));
                items.push(ListItem::new("8. Cancel"));
                
                let items: Vec<ListItem> = items
                    .into_iter()
                    .map(|item| item.style(Style::default().fg(Color::White)))
                    .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Configure GRUB Settings"))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");

                let mut state = ListState::default();
                state.select(Some(*selected));
                f.render_stateful_widget(list, chunks[1], &mut state);
            }
            AppState::EditParameterList { title, params, selected, input_mode, input_buffer } => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(0),
                    ])
                    .split(chunks[1]);

                // Input area
                let input_text = match input_mode {
                    InputMode::None => "".to_string(),
                    InputMode::EditValue(_) => input_buffer.clone(),
                    InputMode::AddName => input_buffer.clone(),
                    InputMode::AddValue(_) => input_buffer.clone(),
                    InputMode::DeleteIndex => input_buffer.clone(),
                };
                let input_title = match input_mode {
                    InputMode::None => "Options".to_string(),
                    InputMode::EditValue(idx) => {
                        if *idx < params.len() {
                            let (name, val) = grub_config::split_parameter(&params[*idx]);
                            format!("Enter new value for {} (current: {})", name, val.unwrap_or_default())
                        } else {
                            "Options".to_string()
                        }
                    }
                    InputMode::AddName => "Enter parameter name".to_string(),
                    InputMode::AddValue(name) => format!("Enter value for {} (or leave empty)", name),
                    InputMode::DeleteIndex => format!("Enter parameter number to delete [1-{}]", params.len()),
                };
                
                let input_widget = Paragraph::new(input_text.as_str())
                    .block(Block::default().borders(Borders::ALL).title(input_title))
                    .style(Style::default().fg(Color::Yellow));
                f.render_widget(input_widget, chunks[0]);

                // Parameter list
                let mut items: Vec<ListItem> = params.iter()
                    .enumerate()
                    .map(|(i, param)| {
                        let (name, value) = grub_config::split_parameter(param);
                        let text = if let Some(val) = value {
                            format!("  {}. {}={}", i + 1, name, val)
                        } else {
                            format!("  {}. {}", i + 1, name)
                        };
                        ListItem::new(text)
                    })
                    .collect();
                
                items.push(ListItem::new("  a. Add new parameter"));
                if !params.is_empty() {
                    items.push(ListItem::new("  d. Delete parameter"));
                }
                items.push(ListItem::new("  s. Save and continue"));
                items.push(ListItem::new("  c. Cancel"));

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(title.as_str()))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");

                let mut state = ListState::default();
                state.select(Some(*selected));
                f.render_stateful_widget(list, chunks[1], &mut state);
            }
            AppState::ViewDefaultEntry => {
                let config = match grub_config::GrubConfig::load() {
                    Ok(c) => c,
                    Err(e) => {
                        let error_text = Paragraph::new(format!("Error loading config: {}", e))
                            .block(Block::default().borders(Borders::ALL).title("Error"))
                            .style(Style::default().fg(Color::Red));
                        f.render_widget(error_text, chunks[1]);
                        return;
                    }
                };

                let entry_ref = if config.grub_default == "\"saved\"" || config.grub_default == "saved" {
                    "GRUB_DEFAULT=saved (current boot entry)".to_string()
                } else {
                    // Try to find entry by path
                    let path_str = config.grub_default.trim_matches('"');
                    let path: Vec<usize> = path_str.split('>')
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    if let Some(entry) = try_get_entry(&self.entry, &path) {
                        entry.name.clone()
                    } else {
                        config.grub_default.clone()
                    }
                };

                let content = vec![
                    Line::from("Current Default Boot Entry:"),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("GRUB_DEFAULT: ", Style::default().fg(Color::Blue)),
                        Span::raw(&config.grub_default),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Entry: ", Style::default().fg(Color::Green)),
                        Span::raw(&entry_ref),
                    ]),
                    Line::from(""),
                    Line::from("Press ESC or Enter to return"),
                ];

                let info = Paragraph::new(content)
                    .block(Block::default().borders(Borders::ALL).title("View Default Boot Entry"))
                    .alignment(Alignment::Left);
                f.render_widget(info, chunks[1]);
            }
            AppState::ConfirmSetDefaultEntry { path, entry_name } => {
                let p_str: String = path.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(">");
                
                let content = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Set '", Style::default().fg(Color::White)),
                        Span::styled(entry_name.as_str(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                        Span::styled("' as permanent default boot entry?", Style::default().fg(Color::White)),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Path: ", Style::default().fg(Color::Blue)),
                        Span::raw(&p_str),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Confirm [", Style::default().fg(Color::White)),
                        Span::styled("Y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        Span::styled("/", Style::default().fg(Color::White)),
                        Span::styled("n", Style::default().fg(Color::Red)),
                        Span::styled("]: ", Style::default().fg(Color::White)),
                    ]),
                ];

                let dialog = Paragraph::new(content)
                    .block(Block::default().borders(Borders::ALL).title("Confirm"))
                    .alignment(Alignment::Center);
                f.render_widget(dialog, chunks[1]);
            }
            AppState::ViewKernelInfo { path, kernel_info } => {
                let entry = get_entry(&self.entry, path);
                let mut content = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Entry: ", Style::default().fg(Color::Blue)),
                        Span::raw(&entry.name),
                    ]),
                    Line::from(""),
                ];
                
                if let Some(info) = kernel_info {
                    content.push(Line::from(vec![
                        Span::styled("Kernel Version: ", Style::default().fg(Color::Green)),
                        Span::raw(&info.version),
                    ]));
                    content.push(Line::from(vec![
                        Span::styled("Release: ", Style::default().fg(Color::Green)),
                        Span::raw(&info.release),
                    ]));
                    content.push(Line::from(vec![
                        Span::styled("Architecture: ", Style::default().fg(Color::Green)),
                        Span::raw(&info.arch),
                    ]));
                    content.push(Line::from(vec![
                        Span::styled("Path: ", Style::default().fg(Color::Green)),
                        Span::raw(&info.path),
                    ]));
                } else {
                    content.push(Line::from(vec![
                        Span::styled("Kernel info not found", Style::default().fg(Color::Yellow)),
                    ]));
                }
                
                let info = Paragraph::new(content)
                    .block(Block::default().borders(Borders::ALL).title("Kernel Information"))
                    .alignment(Alignment::Left);
                f.render_widget(info, chunks[1]);
            }
            AppState::CleanupKernels { kernels, selected } => {
                let items: Vec<ListItem> = if kernels.is_empty() {
                    vec![ListItem::new("No unused kernels found")]
                } else {
                    kernels.iter()
                        .map(|k| {
                            let size_str = kernel_cleanup::format_size(k.size);
                            let status = if k.in_use { "⚠ IN USE" } else { "✓ Safe to remove" };
                            ListItem::new(format!("{} - {} ({}) - {}", 
                                k.version, size_str, k.files.len(), status))
                        })
                        .collect()
                };
                
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Unused Kernels"))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");
                
                let mut state = ListState::default();
                if !kernels.is_empty() {
                    state.select(Some(*selected));
                }
                f.render_stateful_widget(list, chunks[1], &mut state);
            }
            AppState::RenameBootEntry { path, original_name, input_buffer } => {
                let content = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Original Name: ", Style::default().fg(Color::Blue)),
                        Span::raw(original_name),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("New Name: ", Style::default().fg(Color::Green)),
                        Span::raw(input_buffer),
                    ]),
                    Line::from(""),
                    Line::from("Press Enter to save, ESC to cancel"),
                ];
                
                let info = Paragraph::new(content)
                    .block(Block::default().borders(Borders::ALL).title("Rename Boot Entry"))
                    .alignment(Alignment::Left);
                f.render_widget(info, chunks[1]);
            }
            AppState::BackupManager { backups, selected } => {
                let items: Vec<ListItem> = if backups.is_empty() {
                    vec![ListItem::new("No backups found")]
                } else {
                    backups.iter()
                        .map(|b| {
                            let size_str = backup_manager::format_size(b.size);
                            let time_str = backup_manager::format_time(b.modified);
                            let name = b.path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unknown");
                            ListItem::new(format!("{} - {} - {}", name, size_str, time_str))
                        })
                        .collect()
                };
                
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Backup Manager"))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");
                
                let mut state = ListState::default();
                if !backups.is_empty() {
                    state.select(Some(*selected));
                }
                f.render_stateful_widget(list, chunks[1], &mut state);
            }
            AppState::ValidateGrub { result } => {
                let mut content = vec![Line::from("")];
                
                if let Some(r) = result {
                    if r.valid {
                        content.push(Line::from(vec![
                            Span::styled("✓ Configuration is valid", Style::default().fg(Color::Green)),
                        ]));
                    } else {
                        content.push(Line::from(vec![
                            Span::styled("✗ Configuration has errors", Style::default().fg(Color::Red)),
                        ]));
                    }
                    
                    if !r.errors.is_empty() {
                        content.push(Line::from(""));
                        content.push(Line::from(vec![
                            Span::styled("Errors:", Style::default().fg(Color::Red)),
                        ]));
                        for error in &r.errors {
                            content.push(Line::from(format!("  - {}", error)));
                        }
                    }
                    
                    if !r.warnings.is_empty() {
                        content.push(Line::from(""));
                        content.push(Line::from(vec![
                            Span::styled("Warnings:", Style::default().fg(Color::Yellow)),
                        ]));
                        for warning in &r.warnings {
                            content.push(Line::from(format!("  - {}", warning)));
                        }
                    }
                } else {
                    content.push(Line::from(vec![
                        Span::styled("Validating...", Style::default().fg(Color::Yellow)),
                    ]));
                }
                
                let info = Paragraph::new(content)
                    .block(Block::default().borders(Borders::ALL).title("GRUB Configuration Validation"))
                    .alignment(Alignment::Left);
                f.render_widget(info, chunks[1]);
            }
            AppState::BootTimeStats { entries, selected } => {
                let items: Vec<ListItem> = if entries.is_empty() {
                    vec![ListItem::new("No boot time data available")]
                } else {
                    entries.iter()
                        .map(|entry| {
                            let time_str = boot_time::format_boot_time(entry.boot_time);
                            ListItem::new(format!("{} - {} - {}", 
                                entry.kernel_version, time_str, entry.timestamp))
                        })
                        .collect()
                };
                
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Boot Time Statistics"))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");
                
                let mut state = ListState::default();
                if !entries.is_empty() {
                    state.select(Some(*selected));
                }
                f.render_stateful_widget(list, chunks[1], &mut state);
            }
            AppState::EditAllGrubParams { params, selected, input_mode, input_buffer } => {
                let items: Vec<ListItem> = if params.is_empty() {
                    vec![ListItem::new("No parameters found")]
                } else {
                    params.iter()
                        .enumerate()
                        .map(|(idx, (key, value))| {
                            let display_value = match input_mode {
                                GrubConfigInputMode::EditTimeout if idx == *selected => input_buffer.clone(),
                                _ => value.clone(),
                            };
                            ListItem::new(format!("{} = {}", key, display_value))
                        })
                        .collect()
                };
                
                let list = List::new(items)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title("All GRUB Parameters (Enter to edit, Esc to return)"))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");
                
                let mut state = ListState::default();
                if !params.is_empty() {
                    state.select(Some(*selected));
                }
                f.render_stateful_widget(list, chunks[1], &mut state);
            }
            AppState::Message { title, content, message_type } => {
                let color = match message_type {
                    MessageType::Success => Color::Green,
                    MessageType::Error => Color::Red,
                    MessageType::Info => Color::Blue,
                };
                
                let lines: Vec<Line> = content.iter()
                    .map(|line| Line::from(line.as_str()))
                    .collect();

                let message = Paragraph::new(lines)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title(title.as_str())
                        .border_style(Style::default().fg(color)))
                    .alignment(Alignment::Left)
                    .style(Style::default().fg(color));
                f.render_widget(message, chunks[1]);
            }
        }
    }
}

