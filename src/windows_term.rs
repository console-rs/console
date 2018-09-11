use std::char;
use std::io;
use std::mem;
use std::os::windows::io::AsRawHandle;

use winapi;
use winapi::shared::minwindef::DWORD;
use winapi::um::processenv::GetStdHandle;
use winapi::um::winbase::STD_OUTPUT_HANDLE;
use winapi::um::wincon::{
    FillConsoleOutputCharacterA, GetConsoleScreenBufferInfo, SetConsoleCursorPosition,
    CONSOLE_SCREEN_BUFFER_INFO, COORD,
};
use winapi::um::winnt::{CHAR, HANDLE, INT};

use atty;
use kb::Key;
use term::{Term, TermTarget};

pub const DEFAULT_WIDTH: u16 = 79;

pub fn as_handle(term: &Term) -> HANDLE {
    // convert between winapi::um::winnt::HANDLE and std::os::windows::raw::HANDLE
    // which are both c_void. would be nice to find a better way to do this
    unsafe { ::std::mem::transmute(term.as_raw_handle()) }
}

pub fn is_a_terminal(out: &Term) -> bool {
    let stream = match out.target() {
        TermTarget::Stdout => atty::Stream::Stdout,
        TermTarget::Stderr => atty::Stream::Stderr,
        _ => return false,
    };
    atty::is(stream)
}

pub fn terminal_size() -> Option<(u16, u16)> {
    let hand = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    if let Some((_, csbi)) = get_console_screen_buffer_info(hand) {
        Some((
            (csbi.srWindow.Bottom - csbi.srWindow.Top) as u16,
            (csbi.srWindow.Right - csbi.srWindow.Left) as u16,
        ))
    } else {
        None
    }
}

pub fn move_cursor_up(out: &Term, n: usize) -> io::Result<()> {
    if let Some((hand, csbi)) = get_console_screen_buffer_info(as_handle(out)) {
        unsafe {
            SetConsoleCursorPosition(
                hand,
                COORD {
                    X: 0,
                    Y: csbi.dwCursorPosition.Y - n as i16,
                },
            );
        }
    }
    Ok(())
}

pub fn move_cursor_down(out: &Term, n: usize) -> io::Result<()> {
    if let Some((hand, csbi)) = get_console_screen_buffer_info(as_handle(out)) {
        unsafe {
            SetConsoleCursorPosition(
                hand,
                COORD {
                    X: 0,
                    Y: csbi.dwCursorPosition.Y + n as i16,
                },
            );
        }
    }
    Ok(())
}

pub fn clear_line(out: &Term) -> io::Result<()> {
    if let Some((hand, csbi)) = get_console_screen_buffer_info(as_handle(out)) {
        unsafe {
            let width = csbi.srWindow.Right - csbi.srWindow.Left;
            let pos = COORD {
                X: 0,
                Y: csbi.dwCursorPosition.Y,
            };
            let mut written = 0;
            FillConsoleOutputCharacterA(hand, b' ' as CHAR, width as DWORD, pos, &mut written);
            SetConsoleCursorPosition(hand, pos);
        }
    }
    Ok(())
}

fn get_console_screen_buffer_info(hand: HANDLE) -> Option<(HANDLE, CONSOLE_SCREEN_BUFFER_INFO)> {
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
        winapi::um::winuser::VK_LEFT => Key::ArrowLeft,
        winapi::um::winuser::VK_RIGHT => Key::ArrowRight,
        winapi::um::winuser::VK_UP => Key::ArrowUp,
        winapi::um::winuser::VK_DOWN => Key::ArrowDown,
        winapi::um::winuser::VK_RETURN => Key::Enter,
        winapi::um::winuser::VK_ESCAPE => Key::Escape,
        winapi::um::winuser::VK_BACK => Key::Char('\x08'),
        winapi::um::winuser::VK_TAB => Key::Char('\x09'),
        _ => Key::Unknown,
    }
}

pub fn read_secure() -> io::Result<String> {
    let mut rv = String::new();
    loop {
        match read_single_key()? {
            Key::Enter => {
                break;
            }
            Key::Char('\x08') => {
                if rv.len() > 0 {
                    let new_len = rv.len() - 1;
                    rv.truncate(new_len);
                }
            }
            Key::Char(c) => {
                rv.push(c);
            }
            _ => {}
        }
    }
    Ok(rv)
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
