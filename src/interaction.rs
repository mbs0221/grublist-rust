use std::io::{self, Read};
use termion::raw::IntoRawMode;

pub fn get_key_input() -> u8 {
    use termion::raw::IntoRawMode;
    
    // We need to put stdout into raw mode to read from stdin properly
    let stdout = io::stdout();
    let _stdout_raw = stdout.into_raw_mode().unwrap();
    
    let result = loop {
        let mut buffer = [0u8; 1];
        if io::stdin().read_exact(&mut buffer).is_ok() {
            let c = buffer[0];
            
            if c == 27 { // ESC
                // Check if it's an escape sequence or just ESC
                let mut buffer2 = [0u8; 1];
                match io::stdin().read_exact(&mut buffer2) {
                    Ok(_) => {
                        if buffer2[0] == 91 { // [
                            let mut buffer3 = [0u8; 1];
                            if io::stdin().read_exact(&mut buffer3).is_ok() {
                                match buffer3[0] {
                                    65 => break 1, // Up
                                    66 => break 2, // Down
                                    67 => break 3, // Right
                                    68 => break 4, // Left
                                    _ => break 0,
                                }
                            }
                        } else {
                            // ESC followed by non-[ character, treat as ESC
                            break 27;
                        }
                    }
                    Err(_) => {
                        // ESC alone
                        break 27;
                    }
                }
            } else if c == 13 || c == 10 { // Enter
                break 5;
            } else if c == 127 || c == 8 { // Backspace or DEL
                break 127;
            } else if c == b'q' {
                break 6;
            } else if c == b'y' {
                break 7;
            } else if c == b'n' {
                break 8;
            } else if c == b'/' {
                break 47;
            } else if c == b'd' {
                break 100;
            } else if c >= 32 && c <= 126 {
                // Printable ASCII character
                break c;
            } else {
                break 0;
            }
        } else {
            break 0;
        }
    };
    
    // Raw mode is automatically restored when _stdout_raw is dropped
    result
}

