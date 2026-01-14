use std::io::{self, Read, Write};

pub struct MouseEvent {
    pub x: u16,
    pub y: u16,
    pub button: u8, // 0=left, 1=middle, 2=right, 64=release
}

pub enum InputEvent {
    Key(u8),
    MouseClick(MouseEvent),
}

pub fn enable_mouse() {
    // Enable mouse reporting (X10 mode)
    print!("\x1b[?1000h");
    std::io::stdout().flush().unwrap();
}

pub fn disable_mouse() {
    // Disable mouse reporting
    print!("\x1b[?1000l");
    std::io::stdout().flush().unwrap();
}

pub fn get_input() -> InputEvent {
    use termion::raw::IntoRawMode;
    
    // We need to put stdout into raw mode to read from stdin properly
    let stdout = io::stdout();
    let _stdout_raw = stdout.into_raw_mode().unwrap();
    
    let mut buffer = [0u8; 1];
    if io::stdin().read_exact(&mut buffer).is_ok() {
        let c = buffer[0];
        
        if c == 27 { // ESC
            // Check if it's an escape sequence, mouse event, or just ESC
            let mut buffer2 = [0u8; 1];
            match io::stdin().read_exact(&mut buffer2) {
                Ok(_) => {
                    if buffer2[0] == 91 { // [
                        let mut buffer3 = [0u8; 1];
                        if io::stdin().read_exact(&mut buffer3).is_ok() {
                            match buffer3[0] {
                                65 => return InputEvent::Key(1), // Up
                                66 => return InputEvent::Key(2), // Down
                                67 => return InputEvent::Key(3), // Right
                                68 => return InputEvent::Key(4), // Left
                                77 => {
                                    // Mouse event: ESC [ M <button> <x> <y>
                                    let mut button_buf = [0u8; 1];
                                    let mut x_buf = [0u8; 1];
                                    let mut y_buf = [0u8; 1];
                                    
                                    if io::stdin().read_exact(&mut button_buf).is_ok() &&
                                       io::stdin().read_exact(&mut x_buf).is_ok() &&
                                       io::stdin().read_exact(&mut y_buf).is_ok() {
                                        let button = button_buf[0];
                                        let x = x_buf[0] as u16;
                                        let y = y_buf[0] as u16;
                                        
                                        // Only handle left button clicks (button code 32)
                                        if button == 32 {
                                            return InputEvent::MouseClick(MouseEvent {
                                                x: x.saturating_sub(32),
                                                y: y.saturating_sub(32),
                                                button: 0,
                                            });
                                        }
                                    }
                                    return InputEvent::Key(0);
                                }
                                _ => return InputEvent::Key(0),
                            }
                        }
                    } else {
                        // ESC followed by non-[ character, treat as ESC
                        return InputEvent::Key(27);
                    }
                }
                Err(_) => {
                    // ESC alone
                    return InputEvent::Key(27);
                }
            }
        } else if c == 13 || c == 10 { // Enter
            return InputEvent::Key(5);
        } else if c == 127 || c == 8 { // Backspace or DEL
            return InputEvent::Key(127);
        } else if c == b'q' {
            return InputEvent::Key(6);
        } else if c == b'y' {
            return InputEvent::Key(7);
        } else if c == b'n' {
            return InputEvent::Key(8);
        } else if c == b'/' {
            return InputEvent::Key(47);
        } else if c == b'd' {
            return InputEvent::Key(100);
        } else if c >= 32 && c <= 126 {
            // Printable ASCII character
            return InputEvent::Key(c);
        }
    }
    
    InputEvent::Key(0)
}

// Keep the old function for backward compatibility
pub fn get_key_input() -> u8 {
    match get_input() {
        InputEvent::Key(k) => k,
        InputEvent::MouseClick(_) => 0, // Ignore mouse events in old API
    }
}

