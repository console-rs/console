#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

pub const INVALID_HANDLE_VALUE: HANDLE = -1;
pub const MAX_PATH: u32 = 260;
pub type BOOL = i32;
pub type HANDLE = isize;
pub type FILE_INFO_BY_HANDLE_CLASS = i32;
pub const FileNameInfo: FILE_INFO_BY_HANDLE_CLASS = 2;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct COORD {
    pub X: i16,
    pub Y: i16,
}
#[repr(C)]
pub struct CONSOLE_CURSOR_INFO {
    pub dwSize: u32,
    pub bVisible: BOOL,
}

pub type CONSOLE_MODE = u32;

#[derive(Clone)]
#[repr(C)]
pub struct SMALL_RECT {
    pub Left: i16,
    pub Top: i16,
    pub Right: i16,
    pub Bottom: i16,
}
#[derive(Clone)]
#[repr(C)]
pub struct CONSOLE_SCREEN_BUFFER_INFO {
    pub dwSize: COORD,
    pub dwCursorPosition: COORD,
    pub wAttributes: u16,
    pub srWindow: SMALL_RECT,
    pub dwMaximumWindowSize: COORD,
}

#[derive(Copy, Clone)]
#[repr(u32)]
pub enum STD_HANDLE {
    STD_INPUT_HANDLE = 4294967286,
    STD_OUTPUT_HANDLE = 4294967285,
    STD_ERROR_HANDLE = 4294967284,
}
pub const KEY_EVENT: u32 = 1u32;
#[derive(Copy, Clone)]
#[repr(C)]
pub union KEY_EVENT_RECORD_0 {
    pub UnicodeChar: u16,
    pub AsciiChar: u8,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct KEY_EVENT_RECORD {
    pub bKeyDown: BOOL,
    pub wRepeatCount: u16,
    pub wVirtualKeyCode: u16,
    pub wVirtualScanCode: u16,
    pub uChar: KEY_EVENT_RECORD_0,
    pub dwControlKeyState: u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct MOUSE_EVENT_RECORD {
    pub dwMousePosition: COORD,
    pub dwButtonState: u32,
    pub dwControlKeyState: u32,
    pub dwEventFlags: u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct WINDOW_BUFFER_SIZE_RECORD {
    pub dwSize: COORD,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct MENU_EVENT_RECORD {
    pub dwCommandId: u32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct FOCUS_EVENT_RECORD {
    pub bSetFocus: BOOL,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union INPUT_RECORD_0 {
    pub KeyEvent: std::mem::ManuallyDrop<KEY_EVENT_RECORD>,
    pub MouseEvent: std::mem::ManuallyDrop<MOUSE_EVENT_RECORD>,
    pub WindowBufferSizeEvent: std::mem::ManuallyDrop<WINDOW_BUFFER_SIZE_RECORD>,
    pub MenuEvent: std::mem::ManuallyDrop<MENU_EVENT_RECORD>,
    pub FocusEvent: std::mem::ManuallyDrop<FOCUS_EVENT_RECORD>,
}
#[repr(C)]
pub struct INPUT_RECORD {
    pub EventType: u16,
    pub Event: INPUT_RECORD_0,
}

pub type VIRTUAL_KEY = u16;
pub const VK_BACK: VIRTUAL_KEY = 8;
pub const VK_TAB: VIRTUAL_KEY = 9;
pub const VK_RETURN: VIRTUAL_KEY = 13;
pub const VK_SHIFT: VIRTUAL_KEY = 16;
pub const VK_MENU: VIRTUAL_KEY = 18;
pub const VK_ESCAPE: VIRTUAL_KEY = 27;
pub const VK_END: VIRTUAL_KEY = 35;
pub const VK_HOME: VIRTUAL_KEY = 36;
pub const VK_LEFT: VIRTUAL_KEY = 37;
pub const VK_UP: VIRTUAL_KEY = 38;
pub const VK_RIGHT: VIRTUAL_KEY = 39;
pub const VK_DOWN: VIRTUAL_KEY = 40;
pub const VK_DELETE: VIRTUAL_KEY = 46;

#[repr(C)]
pub struct FILE_NAME_INFO {
    pub FileNameLength: u32,
    pub FileName: [u16; 1],
}

#[link(name = "kernel32")]
extern "system" {
    pub fn GetFileInformationByHandleEx(
        hFile: HANDLE,
        FileInformationClass: FILE_INFO_BY_HANDLE_CLASS,
        lpFileInformation: *mut std::ffi::c_void,
        dwBufferSize: u32,
    ) -> BOOL;
    pub fn FillConsoleOutputAttribute(
        hConsoleOutput: HANDLE,
        wAttribute: u16,
        nLength: u32,
        dwWriteCoord: COORD,
        lpNumberOfAttrsWritten: *mut u32,
    ) -> BOOL;
    pub fn FillConsoleOutputCharacterA(
        hConsoleOutput: HANDLE,
        cCharacter: u8,
        nLength: u32,
        dwWriteCoord: COORD,
        lpNumberOfCharsWritten: *mut u32,
    ) -> BOOL;
    pub fn GetConsoleCursorInfo(
        hConsoleOutput: HANDLE,
        lpConsoleCursorInfo: *mut CONSOLE_CURSOR_INFO,
    ) -> BOOL;
    pub fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *mut CONSOLE_MODE) -> BOOL;
    pub fn GetConsoleScreenBufferInfo(
        hConsoleOutput: HANDLE,
        lpConsoleScreenBufferInfo: *mut CONSOLE_SCREEN_BUFFER_INFO,
    ) -> BOOL;
    pub fn GetNumberOfConsoleInputEvents(hConsoleInput: HANDLE, lpNumberOfEvents: *mut u32)
        -> BOOL;
    pub fn GetStdHandle(nStdHandle: STD_HANDLE) -> HANDLE;
    pub fn ReadConsoleInputW(
        hConsoleInput: HANDLE,
        lpBuffer: *mut INPUT_RECORD,
        nLength: u32,
        lpNumberOfEventsRead: *mut u32,
    ) -> BOOL;
    pub fn SetConsoleCursorInfo(
        hConsoleOutput: HANDLE,
        lpConsoleCursorInfo: *const CONSOLE_CURSOR_INFO,
    ) -> BOOL;
    pub fn SetConsoleCursorPosition(hConsoleOutput: HANDLE, dwCursorPosition: COORD) -> BOOL;
    pub fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: CONSOLE_MODE) -> BOOL;
    pub fn SetConsoleTitleW(lpConsoleTitle: *const u16) -> BOOL;
}
