use std::char;
use std::io;
use std::mem;
use std::os::windows::io::AsRawHandle;
use std::slice;

use winapi;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::MAX_PATH;
use winapi::um::fileapi::FILE_NAME_INFO;
use winapi::um::minwinbase::FileNameInfo;
use winapi::um::processenv::GetStdHandle;
use winapi::um::winbase::GetFileInformationByHandleEx;
use winapi::um::winbase::STD_OUTPUT_HANDLE;
use winapi::um::wincon::{
    FillConsoleOutputCharacterA, GetConsoleScreenBufferInfo, SetConsoleCursorPosition,
    CONSOLE_SCREEN_BUFFER_INFO, COORD,
};
use winapi::um::winnt::{CHAR, HANDLE, INT, WCHAR};

use atty;
use common_term;
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
    if msys_tty_on(out) {
        return common_term::move_cursor_up(out, n);
    }
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
    if msys_tty_on(out) {
        return common_term::move_cursor_down(out, n);
    }
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
    if msys_tty_on(out) {
        return common_term::clear_line(out);
    }
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

/// Returns true if there is an MSYS tty on the given handle.
fn msys_tty_on(term: &Term) -> bool {
    let handle = term.as_raw_handle();
    unsafe {
        let size = mem::size_of::<FILE_NAME_INFO>();
        let mut name_info_bytes = vec![0u8; size + MAX_PATH * mem::size_of::<WCHAR>()];
        let res = GetFileInformationByHandleEx(
            handle as *mut _,
            FileNameInfo,
            &mut *name_info_bytes as *mut _ as *mut c_void,
            name_info_bytes.len() as u32,
        );
        if res == 0 {
            return false;
        }
        let name_info: &FILE_NAME_INFO = &*(name_info_bytes.as_ptr() as *const FILE_NAME_INFO);
        let s = slice::from_raw_parts(
            name_info.FileName.as_ptr(),
            name_info.FileNameLength as usize / 2,
        );
        let name = String::from_utf16_lossy(s);
        // This checks whether 'pty' exists in the file name, which indicates that
        // a pseudo-terminal is attached. To mitigate against false positives
        // (e.g., an actual file name that contains 'pty'), we also require that
        // either the strings 'msys-' or 'cygwin-' are in the file name as well.)
        let is_msys = name.contains("msys-") || name.contains("cygwin-");
        let is_pty = name.contains("-pty");
        is_msys && is_pty
    }
}
