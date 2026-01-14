use std::io::{self, Read};
use termion::raw::IntoRawMode;

pub fn get_key_input() -> u8 {
    // We need to put stdout into raw mode to read from stdin properly
    let stdout = io::stdout();
    let _stdout_raw = stdout.into_raw_mode().unwrap();
    
    let result = loop {
        let mut buffer = [0u8; 1];
        if io::stdin().read_exact(&mut buffer).is_ok() {
            let c = buffer[0];
            
            if c == 27 { // ESC
                let mut buffer2 = [0u8; 1];
                if io::stdin().read_exact(&mut buffer2).is_ok() {
                    if buffer2[0] != 91 {
                        break 0;
                    }
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
                }
            } else if c == 13 || c == 10 { // Enter
                break 5;
            } else if c == b'q' {
                break 6;
            } else if c == b'y' {
                break 7;
            } else if c == b'n' {
                break 8;
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

