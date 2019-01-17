use std::char;
use std::io;
use std::mem;
use std::os::windows::io::AsRawHandle;
use std::slice;

use encode_unicode::error::InvalidUtf16Tuple;
use encode_unicode::CharExt;
use winapi;
use winapi::ctypes::c_void;
use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::MAX_PATH;
use winapi::um::consoleapi::{GetNumberOfConsoleInputEvents, ReadConsoleInputW};
use winapi::um::fileapi::FILE_NAME_INFO;
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
use winapi::um::minwinbase::FileNameInfo;
use winapi::um::processenv::GetStdHandle;
use winapi::um::winbase::GetFileInformationByHandleEx;
use winapi::um::winbase::{STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};
use winapi::um::wincon::{
    FillConsoleOutputCharacterA, GetConsoleScreenBufferInfo, SetConsoleCursorPosition,
    CONSOLE_SCREEN_BUFFER_INFO, COORD, INPUT_RECORD, KEY_EVENT, KEY_EVENT_RECORD,
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

pub fn clear_screen(out: &Term) -> io::Result<()> {
    if msys_tty_on(out) {
        return common_term::clear_screen(out);
    }
    if let Some((hand, csbi)) = get_console_screen_buffer_info(as_handle(out)) {
        unsafe {
            let cells = csbi.dwSize.X * csbi.dwSize.Y;
            let pos = COORD { X: 0, Y: 0 };
            let mut written = 0;
            FillConsoleOutputCharacterA(hand, b' ' as CHAR, cells as DWORD, pos, &mut written);
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
    let key_event = read_key_event()?;

    let unicode_char = unsafe { *key_event.uChar.UnicodeChar() };
    if unicode_char == 0 {
        return Ok(key_from_key_code(key_event.wVirtualKeyCode as INT));
    } else {
        // This is a unicode character, in utf-16. Try to decode it by itself.
        match char::from_utf16_tuple((unicode_char, None)) {
            Ok(c) => {
                // Maintain backward compatibility. The previous implementation (_getwch()) would return
                // a special keycode for `Enter`, while ReadConsoleInputW() prefers to use '\r'.
                if c == '\r' {
                    Ok(Key::Enter)
                } else {
                    Ok(Key::Char(c))
                }
            }
            // This is part of a surrogate pair. Try to read the second half.
            Err(InvalidUtf16Tuple::MissingSecond) => {
                // Confirm that there is a next character to read.
                if get_key_event_count()? == 0 {
                    let message = format!(
                        "Read invlid utf16 {}: {}",
                        unicode_char,
                        InvalidUtf16Tuple::MissingSecond
                    );
                    return Err(io::Error::new(io::ErrorKind::InvalidData, message));
                }

                // Read the next character.
                let next_event = read_key_event()?;
                let next_surrogate = unsafe { *next_event.uChar.UnicodeChar() };

                // Attempt to decode it.
                match char::from_utf16_tuple((unicode_char, Some(next_surrogate))) {
                    Ok(c) => Ok(Key::Char(c)),

                    // Return an InvalidData error. This is the recommended value for UTF-related I/O errors.
                    // (This error is given when reading a non-UTF8 file into a String, for example.)
                    Err(e) => {
                        let message = format!(
                            "Read invalid surrogate pair ({}, {}): {}",
                            unicode_char, next_surrogate, e
                        );
                        Err(io::Error::new(io::ErrorKind::InvalidData, message))
                    }
                }
            }

            // Return an InvalidData error. This is the recommended value for UTF-related I/O errors.
            // (This error is given when reading a non-UTF8 file into a String, for example.)
            Err(e) => {
                let message = format!("Read invalid utf16 {}: {}", unicode_char, e);
                Err(io::Error::new(io::ErrorKind::InvalidData, message))
            }
        }
    }
}

fn get_stdin_handle() -> io::Result<HANDLE> {
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    if handle == INVALID_HANDLE_VALUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(handle)
    }
}

/// Get the number of pending events in the ReadConsoleInput queue. Note that while
/// these aren't necessarily key events, the only way that multiple events can be
/// put into the queue simultaneously is if a unicode character spanning multiple u16's
/// is read.
///
/// Therefore, this is accurate as long as at least one KEY_EVENT has already been read.
fn get_key_event_count() -> io::Result<DWORD> {
    let handle = get_stdin_handle()?;
    let mut event_count: DWORD = unsafe { mem::zeroed() };

    let success = unsafe { GetNumberOfConsoleInputEvents(handle, &mut event_count) };
    if success == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(event_count)
    }
}

fn read_key_event() -> io::Result<KEY_EVENT_RECORD> {
    let handle = get_stdin_handle()?;
    let mut buffer: INPUT_RECORD = unsafe { mem::zeroed() };

    let mut events_read: DWORD = unsafe { mem::zeroed() };

    let mut key_event: KEY_EVENT_RECORD;
    loop {
        let success = unsafe { ReadConsoleInputW(handle, &mut buffer, 1, &mut events_read) };
        if success == 0 {
            return Err(io::Error::last_os_error());
        }
        if events_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "ReadConsoleInput returned no events, instead of waiting for an event",
            ));
        }

        if events_read == 1 && buffer.EventType != KEY_EVENT {
            // This isn't a key event; ignore it.
            continue;
        }

        key_event = unsafe { mem::transmute(buffer.Event) };

        if key_event.bKeyDown == 0 {
            // This is a key being released; ignore it.
            continue;
        }

        return Ok(key_event);
    }
}

pub fn wants_emoji() -> bool {
    false
}

/// Returns true if there is an MSYS tty on the given handle.
pub fn msys_tty_on(term: &Term) -> bool {
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
