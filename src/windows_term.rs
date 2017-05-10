use std::io;
use std::mem;
use std::char;
use std::os::windows::io::AsRawHandle;

use winapi;
use winapi::{INT, CHAR, DWORD, HANDLE, STD_OUTPUT_HANDLE,
             CONSOLE_SCREEN_BUFFER_INFO, COORD};
use kernel32::{GetConsoleScreenBufferInfo, GetStdHandle,
               GetConsoleMode, SetConsoleCursorPosition,
               FillConsoleOutputCharacterA};

use term::Term;
use kb::Key;

pub const DEFAULT_WIDTH: u16 = 79;


pub fn is_a_terminal(out: &Term) -> bool {
    unsafe {
        let mut tmp = 0;
        GetConsoleMode(out.as_raw_handle(), &mut tmp) != 0
    }
}

pub fn terminal_size() -> Option<(u16, u16)> {
    let hand = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    if let Some((_, csbi)) = get_console_screen_buffer_info(hand) {
        Some(((csbi.srWindow.Bottom - csbi.srWindow.Top) as u16,
              (csbi.srWindow.Right - csbi.srWindow.Left) as u16))
    } else {
        None
    }
}

pub fn move_cursor_up(out: &Term, n: usize) -> io::Result<()> {
    if let Some((hand, csbi)) = get_console_screen_buffer_info(out.as_raw_handle()) {
        unsafe {
            SetConsoleCursorPosition(hand, COORD {
                X: 0,
                Y: csbi.dwCursorPosition.Y - n as i16,
            });
        }
    }
    Ok(())
}

pub fn move_cursor_down(out: &Term, n: usize) -> io::Result<()> {
    if let Some((hand, csbi)) = get_console_screen_buffer_info(out.as_raw_handle()) {
        unsafe {
            SetConsoleCursorPosition(hand, COORD {
                X: 0,
                Y: csbi.dwCursorPosition.Y + n as i16,
            });
        }
    }
    Ok(())
}

pub fn clear_line(out: &Term) -> io::Result<()> {
    if let Some((hand, csbi)) = get_console_screen_buffer_info(out.as_raw_handle()) {
        unsafe {
            let width = csbi.srWindow.Right - csbi.srWindow.Left;
            let pos = COORD {
                X: 0,
                Y: csbi.dwCursorPosition.Y,
            };
            let mut written = 0;
            FillConsoleOutputCharacterA(hand, b' ' as CHAR,
                                        width as DWORD, pos, &mut written);
            SetConsoleCursorPosition(hand, pos);
        }
    }
    Ok(())
}

fn get_console_screen_buffer_info(hand: HANDLE)
    -> Option<(HANDLE, CONSOLE_SCREEN_BUFFER_INFO)>
{
    let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = unsafe { mem::zeroed() };
    match unsafe { GetConsoleScreenBufferInfo(hand, &mut csbi) } {
        0 => None,
        _ => Some((hand, csbi)),
    }
}

extern "C" {
    fn _getwch() -> INT;
}

pub fn key_from_key_code(code: INT) -> Key {
    match code {
        winapi::VK_LEFT => Key::ArrowLeft,
        winapi::VK_RIGHT => Key::ArrowRight,
        winapi::VK_UP => Key::ArrowUp,
        winapi::VK_DOWN => Key::ArrowDown,
        winapi::VK_RETURN => Key::Enter,
        winapi::VK_ESCAPE => Key::Escape,
        _ => Key::Unknown,
    }
}

pub fn read_single_key() -> io::Result<Key> {
    unsafe {
        let c = _getwch();
        // this is bullshit, we should convert such thing into errors
        if c == 0 || c == 0xe0 {
            Ok(key_from_key_code(_getwch()))
        } else {
            Ok(Key::Char(char::from_u32(c as u32).unwrap_or('\x00')))
        }
    }
}

pub fn wants_emoji() -> bool {
    false
}
