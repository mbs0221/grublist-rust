mod colorprint;
mod grub;
mod grub_config;

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
    search_query: String,
    search_results: Vec<Vec<usize>>,
    search_selected: usize,
    bcolors: colorprint::Bcolors,
}

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
    },
    SelectBootEntrySearch {
        path: Vec<usize>,
        query: String,
        results: Vec<Vec<usize>>,
        selected: usize,
    },
    ConfigureKernelParams {
        selected: usize,
        linux_params: Vec<String>,
        linux_default_params: Vec<String>,
    },
    EditParameterList {
        title: String,
        params: Vec<String>,
        selected: usize,
        input_mode: InputMode,
        input_buffer: String,
    },
    ViewDefaultEntry,
    ConfigureTimeout {
        selected: usize,
        timeout: String,
        timeout_style: String,
        input_mode: TimeoutInputMode,
        input_buffer: String,
    },
    ConfirmSetDefaultEntry {
        path: Vec<usize>,
        entry_name: String,
    },
    Message {
        title: String,
        content: Vec<String>,
        message_type: MessageType,
    },
}

enum MessageType {
    Success,
    Error,
    Info,
}

#[derive(PartialEq, Clone)]
enum TimeoutInputMode {
    None,
    EditTimeout,
    SelectTimeoutStyle,
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
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            bcolors: colorprint::Bcolors::new(),
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
                    AppState::SelectBootEntry { path, selected } => (2, *selected),
                    AppState::SelectBootEntrySearch { path: _, query: _, results: _, selected } => (3, *selected),
                    AppState::ConfigureKernelParams { selected, .. } => (4, *selected),
                    AppState::EditParameterList { selected, .. } => (5, *selected),
                    AppState::ViewDefaultEntry => (6, 0),
                    AppState::ConfigureTimeout { selected, .. } => (7, *selected),
                    AppState::ConfirmSetDefaultEntry { .. } => (8, 0),
                    AppState::Message { .. } => (9, 0),
                };

                match state_snapshot.0 {
                    0 => { // MainMenu
                        match key.code {
                            KeyCode::Esc => break,
                            KeyCode::Up => {
                                if let AppState::MainMenu { selected } = &mut self.state {
                                    if *selected == 0 {
                                        *selected = 3;
                                    } else {
                                        *selected -= 1;
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::MainMenu { selected } = &mut self.state {
                                    *selected = (*selected + 1) % 4;
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
                                self.state = AppState::MainMenu { selected: 0 };
                            }
                            KeyCode::Up => {
                                if let AppState::SelectBootEntry { path, selected } = &mut self.state {
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
                                if let AppState::SelectBootEntry { path, selected } = &mut self.state {
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
                                if let AppState::SelectBootEntry { path, selected } = &self.state {
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
                                            self.state = AppState::ConfirmSetDefaultEntry {
                                                path: result_path,
                                                entry_name,
                                            };
                                        } else if child.entry_type == EntryType::Submenu {
                                            if let AppState::SelectBootEntry { path: p, selected: s } = &mut self.state {
                                                p.push(state_snapshot.1);
                                                *s = 0;
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Left => {
                                if let AppState::SelectBootEntry { path, selected } = &mut self.state {
                                    if !path.is_empty() {
                                        path.pop();
                                        *selected = 0;
                                    } else {
                                        // If at root level, go back to main menu
                                        self.state = AppState::MainMenu { selected: 0 };
                                    }
                                }
                            }
                            _ => {
                                if let Some(c) = Self::key_to_char(&key) {
                                    if let AppState::SelectBootEntry { path, .. } = &self.state {
                                        self.start_boot_entry_search(c, path.clone());
                                    }
                                }
                            }
                        }
                    }
                    3 => { // SelectBootEntrySearch
                        match key.code {
                            KeyCode::Esc => {
                                if let AppState::SelectBootEntrySearch { path, .. } = &self.state {
                                    let path = path.clone();
                                    self.state = AppState::SelectBootEntry {
                                        path,
                                        selected: 0,
                                    };
                                }
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
                                        self.state = AppState::ConfirmSetDefaultEntry {
                                            path: result_path,
                                            entry_name,
                                        };
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
                    4 => { // ConfigureKernelParams
                        match key.code {
                            KeyCode::Esc | KeyCode::Left => {
                                self.state = AppState::MainMenu { selected: 0 };
                            }
                            KeyCode::Up => {
                                if let AppState::ConfigureKernelParams { selected, .. } = &mut self.state {
                                    if *selected == 0 {
                                        *selected = 3;
                                    } else {
                                        *selected -= 1;
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::ConfigureKernelParams { selected, .. } = &mut self.state {
                                    *selected = (*selected + 1) % 4;
                                }
                            }
                            KeyCode::Enter | KeyCode::Right => {
                                if let AppState::ConfigureKernelParams { selected, linux_params, linux_default_params } = &mut self.state {
                                    match *selected {
                                        0 => {
                                            // Edit GRUB_CMDLINE_LINUX
                                            let params = linux_params.clone();
                                            self.state = AppState::EditParameterList {
                                                title: "Edit GRUB_CMDLINE_LINUX".to_string(),
                                                params,
                                                selected: 0,
                                                input_mode: InputMode::None,
                                                input_buffer: String::new(),
                                            };
                                        }
                                        1 => {
                                            // Edit GRUB_CMDLINE_LINUX_DEFAULT
                                            let params = linux_default_params.clone();
                                            self.state = AppState::EditParameterList {
                                                title: "Edit GRUB_CMDLINE_LINUX_DEFAULT".to_string(),
                                                params,
                                                selected: 0,
                                                input_mode: InputMode::None,
                                                input_buffer: String::new(),
                                            };
                                        }
                                        2 => {
                                            // Save and exit
                                            let mut config = match grub_config::GrubConfig::load() {
                                                Ok(c) => c,
                                                Err(_) => {
                                                    self.state = AppState::MainMenu { selected: 0 };
                                                    continue;
                                                }
                                            };
                                            config.grub_cmdline_linux = grub_config::join_parameters(linux_params);
                                            config.grub_cmdline_linux_default = grub_config::join_parameters(linux_default_params);
                                            
                                            match config.save() {
                                                Ok(_) => {
                                                    self.state = AppState::MainMenu { selected: 0 };
                                                }
                                                Err(_) => {
                                                    // TODO: Show error
                                                    self.state = AppState::MainMenu { selected: 0 };
                                                }
                                            }
                                        }
                                        3 => {
                                            // Cancel
                                            self.state = AppState::MainMenu { selected: 0 };
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
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
                                        // Return to kernel params menu with updated params
                                        let params = params.clone();
                                        let is_linux = title == "Edit GRUB_CMDLINE_LINUX";
                                        
                                        match grub_config::GrubConfig::load() {
                                            Ok(mut config) => {
                                                let linux_params = grub_config::parse_parameters(&config.grub_cmdline_linux);
                                                let linux_default_params = grub_config::parse_parameters(&config.grub_cmdline_linux_default);
                                                
                                                if is_linux {
                                                    self.state = AppState::ConfigureKernelParams {
                                                        selected: 0,
                                                        linux_params: params,
                                                        linux_default_params,
                                                    };
                                                } else {
                                                    self.state = AppState::ConfigureKernelParams {
                                                        selected: 1,
                                                        linux_params,
                                                        linux_default_params: params,
                                                    };
                                                }
                                            }
                                            Err(_) => {
                                                self.state = AppState::MainMenu { selected: 0 };
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
                                                                    self.state = AppState::ConfigureKernelParams {
                                                                        selected: 0,
                                                                        linux_params: params,
                                                                        linux_default_params,
                                                                    };
                                                                } else {
                                                                    self.state = AppState::ConfigureKernelParams {
                                                                        selected: 1,
                                                                        linux_params,
                                                                        linux_default_params: params,
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
                                                                
                                                                self.state = AppState::ConfigureKernelParams {
                                                                    selected: if is_linux { 0 } else { 1 },
                                                                    linux_params,
                                                                    linux_default_params,
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
                                                                    self.state = AppState::ConfigureKernelParams {
                                                                        selected: 0,
                                                                        linux_params: params,
                                                                        linux_default_params,
                                                                    };
                                                                } else {
                                                                    self.state = AppState::ConfigureKernelParams {
                                                                        selected: 1,
                                                                        linux_params,
                                                                        linux_default_params: params,
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
                                                            
                                                            self.state = AppState::ConfigureKernelParams {
                                                                selected: if is_linux { 0 } else { 1 },
                                                                linux_params,
                                                                linux_default_params,
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
                                self.state = AppState::MainMenu { selected: 0 };
                            }
                            _ => {}
                        }
                    }
                    7 => { // ConfigureTimeout
                        match key.code {
                            KeyCode::Esc | KeyCode::Left => {
                                if let AppState::ConfigureTimeout { input_mode, input_buffer, .. } = &mut self.state {
                                    if *input_mode != TimeoutInputMode::None {
                                        *input_mode = TimeoutInputMode::None;
                                        *input_buffer = String::new();
                                    } else {
                                        self.state = AppState::MainMenu { selected: 0 };
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if let AppState::ConfigureTimeout { selected, input_mode, .. } = &mut self.state {
                                    if *input_mode == TimeoutInputMode::None {
                                        if *selected == 0 {
                                            *selected = 3;
                                        } else {
                                            *selected -= 1;
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let AppState::ConfigureTimeout { selected, input_mode, .. } = &mut self.state {
                                    if *input_mode == TimeoutInputMode::None {
                                        *selected = (*selected + 1) % 4;
                                    }
                                }
                            }
                            KeyCode::Enter | KeyCode::Right => {
                                let is_right_key = matches!(key.code, KeyCode::Right);
                                if let AppState::ConfigureTimeout { selected, timeout, timeout_style, input_mode, input_buffer } = &mut self.state {
                                    let current_input_mode = input_mode.clone();
                                    match current_input_mode {
                                        TimeoutInputMode::None => {
                                            match *selected {
                                                0 => {
                                                    // Edit timeout
                                                    *input_mode = TimeoutInputMode::EditTimeout;
                                                    *input_buffer = timeout.clone();
                                                }
                                                1 => {
                                                    // Edit timeout style
                                                    *input_mode = TimeoutInputMode::SelectTimeoutStyle;
                                                    *input_buffer = String::new();
                                                }
                                                2 => {
                                                    // Save
                                                    let mut config = match grub_config::GrubConfig::load() {
                                                        Ok(c) => c,
                                                        Err(e) => {
                                                            self.state = AppState::Message {
                                                                title: "Error".to_string(),
                                                                content: vec![
                                                                    format!("Error loading config: {}", e),
                                                                ],
                                                                message_type: MessageType::Error,
                                                            };
                                                            continue;
                                                        }
                                                    };
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
                                                                content: vec![
                                                                    format!("Error saving configuration: {}", e),
                                                                ],
                                                                message_type: MessageType::Error,
                                                            };
                                                        }
                                                    }
                                                }
                                                3 => {
                                                    // Cancel
                                                    self.state = AppState::MainMenu { selected: 0 };
                                                }
                                                _ => {}
                                            }
                                        }
                                        TimeoutInputMode::EditTimeout => {
                                            if !input_buffer.trim().is_empty() {
                                                *timeout = input_buffer.trim().to_string();
                                            }
                                            
                                            // If Right key was pressed, move to timeout style editing
                                            if is_right_key {
                                                *selected = 1;
                                                *input_mode = TimeoutInputMode::SelectTimeoutStyle;
                                                *input_buffer = String::new();
                                            } else {
                                                // Enter key, just exit edit mode
                                                *input_mode = TimeoutInputMode::None;
                                                *input_buffer = String::new();
                                            }
                                        }
                                        TimeoutInputMode::SelectTimeoutStyle => {
                                            let style = input_buffer.trim().to_lowercase();
                                            if style == "menu" || style == "hidden" || style == "countdown" {
                                                *timeout_style = style;
                                            }
                                            
                                            // If Right key was pressed, could move to save, but for now just exit
                                            *input_mode = TimeoutInputMode::None;
                                            *input_buffer = String::new();
                                        }
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                if let AppState::ConfigureTimeout { input_mode, input_buffer, .. } = &mut self.state {
                                    if *input_mode != TimeoutInputMode::None {
                                        input_buffer.pop();
                                    }
                                }
                            }
                            _ => {
                                if let AppState::ConfigureTimeout { input_mode, input_buffer, .. } = &mut self.state {
                                    if *input_mode != TimeoutInputMode::None {
                                        if let Some(c) = Self::key_to_char(&key) {
                                            input_buffer.push(c);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    8 => { // ConfirmSetDefaultEntry
                        match key.code {
                            KeyCode::Esc => {
                                self.state = AppState::MainMenu { selected: 0 };
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
                                self.state = AppState::MainMenu { selected: 0 };
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
                self.state = AppState::SelectBootEntry {
                    path: vec![],
                    selected: 0,
                };
            }
            1 => {
                // Configure Kernel Parameters
                match grub_config::GrubConfig::load() {
                    Ok(config) => {
                        let linux_params = grub_config::parse_parameters(&config.grub_cmdline_linux);
                        let linux_default_params = grub_config::parse_parameters(&config.grub_cmdline_linux_default);
                        self.state = AppState::ConfigureKernelParams {
                            selected: 0,
                            linux_params,
                            linux_default_params,
                        };
                    }
                    Err(e) => {
                        // TODO: Show error message
                        return Ok(());
                    }
                }
            }
            2 => {
                // Configure GRUB Timeout
                match grub_config::GrubConfig::load() {
                    Ok(config) => {
                        self.state = AppState::ConfigureTimeout {
                            selected: 0,
                            timeout: config.grub_timeout,
                            timeout_style: config.grub_timeout_style,
                            input_mode: TimeoutInputMode::None,
                            input_buffer: String::new(),
                        };
                    }
                    Err(_) => {
                        return Ok(());
                    }
                }
            }
            3 => {
                // View Default Boot Entry
                self.state = AppState::ViewDefaultEntry;
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
        self.state = AppState::SelectBootEntrySearch {
            path,
            query,
            results,
            selected: 0,
        };
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
                Span::raw(" v0.1.5                        "),
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
                    ListItem::new("⚙ Configure Kernel Parameters"),
                    ListItem::new("⏱ Configure GRUB Timeout"),
                    ListItem::new("👁 View Default Boot Entry"),
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
            AppState::SelectBootEntry { path, selected } => {
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
            AppState::ConfigureKernelParams { selected, linux_params, linux_default_params } => {
                let items: Vec<ListItem> = vec![
                    ListItem::new(format!("1. GRUB_CMDLINE_LINUX ({})", 
                        if linux_params.is_empty() { "empty".to_string() } 
                        else { format!("{} parameters", linux_params.len()) })),
                    ListItem::new(format!("2. GRUB_CMDLINE_LINUX_DEFAULT ({})", 
                        if linux_default_params.is_empty() { "empty".to_string() } 
                        else { format!("{} parameters", linux_default_params.len()) })),
                    ListItem::new("3. Save and exit"),
                    ListItem::new("4. Cancel"),
                ]
                .into_iter()
                .map(|item| item.style(Style::default().fg(Color::White)))
                .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Configure Kernel Boot Parameters"))
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
            AppState::ConfigureTimeout { selected, timeout, timeout_style, input_mode, input_buffer } => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(0),
                    ])
                    .split(chunks[1]);

                // Input area
                let input_text = match input_mode {
                    TimeoutInputMode::None => "".to_string(),
                    TimeoutInputMode::EditTimeout => input_buffer.clone(),
                    TimeoutInputMode::SelectTimeoutStyle => input_buffer.clone(),
                };
                let input_title = match input_mode {
                    TimeoutInputMode::None => "Options".to_string(),
                    TimeoutInputMode::EditTimeout => {
                        format!("Enter timeout in seconds (current: {}, -1 for no timeout)", timeout)
                    }
                    TimeoutInputMode::SelectTimeoutStyle => {
                        format!("Enter timeout style (current: {}, options: menu/hidden/countdown)", timeout_style)
                    }
                };
                
                let input_widget = Paragraph::new(input_text.as_str())
                    .block(Block::default().borders(Borders::ALL).title(input_title))
                    .style(Style::default().fg(Color::Yellow));
                f.render_widget(input_widget, chunks[0]);

                // Menu list
                let items: Vec<ListItem> = vec![
                    ListItem::new(format!("1. GRUB_TIMEOUT: {}", timeout)),
                    ListItem::new(format!("2. GRUB_TIMEOUT_STYLE: {}", timeout_style)),
                    ListItem::new("3. Save and exit"),
                    ListItem::new("4. Cancel"),
                ]
                .into_iter()
                .map(|item| item.style(Style::default().fg(Color::White)))
                .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Configure GRUB Timeout"))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol(">> ");

                let mut state = ListState::default();
                if *input_mode == TimeoutInputMode::None {
                    state.select(Some(*selected));
                }
                f.render_stateful_widget(list, chunks[1], &mut state);
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

