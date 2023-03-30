// Uncomment this to regenerate bindings
// #[test]
// fn regenerate_bindings() {
//     let apis = [
//         "Windows.Win32.Foundation.BOOL",
//         "Windows.Win32.Foundation.HANDLE",
//         "Windows.Win32.Foundation.MAX_PATH",
//         "Windows.Win32.Foundation.INVALID_HANDLE_VALUE",
//         "Windows.Win32.System.Console.COORD",
//         "Windows.Win32.System.Console.FillConsoleOutputAttribute",
//         "Windows.Win32.System.Console.FillConsoleOutputCharacterA",
//         "Windows.Win32.System.Console.CONSOLE_CURSOR_INFO",
//         "Windows.Win32.System.Console.GetConsoleCursorInfo",
//         "Windows.Win32.System.Console.CONSOLE_MODE",
//         "Windows.Win32.System.Console.GetConsoleMode",
//         "Windows.Win32.System.Console.SMALL_RECT",
//         "Windows.Win32.System.Console.CONSOLE_SCREEN_BUFFER_INFO",
//         "Windows.Win32.System.Console.GetConsoleScreenBufferInfo",
//         "Windows.Win32.System.Console.GetNumberOfConsoleInputEvents",
//         "Windows.Win32.System.Console.STD_HANDLE",
//         "Windows.Win32.System.Console.GetStdHandle",
//         "Windows.Win32.System.Console.KEY_EVENT",
//         "Windows.Win32.System.Console.KEY_EVENT_RECORD",
//         "Windows.Win32.System.Console.MOUSE_EVENT_RECORD",
//         "Windows.Win32.System.Console.WINDOW_BUFFER_SIZE_RECORD",
//         "Windows.Win32.System.Console.MENU_EVENT_RECORD",
//         "Windows.Win32.System.Console.FOCUS_EVENT_RECORD",
//         "Windows.Win32.System.Console.INPUT_RECORD",
//         "Windows.Win32.System.Console.ReadConsoleInputW",
//         "Windows.Win32.System.Console.CONSOLE_CHARACTER_ATTRIBUTES",
//         "Windows.Win32.System.Console.SetConsoleTextAttribute",
//         "Windows.Win32.System.Console.SetConsoleCursorInfo",
//         "Windows.Win32.System.Console.SetConsoleCursorPosition",
//         "Windows.Win32.System.Console.SetConsoleMode",
//         "Windows.Win32.System.Console.SetConsoleTitleW",
//         "Windows.Win32.Storage.FileSystem.FILE_NAME_INFO",
//         "Windows.Win32.Storage.FileSystem.FILE_INFO_BY_HANDLE_CLASS",
//         "Windows.Win32.Storage.FileSystem.GetFileInformationByHandleEx",
//         "Windows.Win32.UI.Input.KeyboardAndMouse.VIRTUAL_KEY",
//     ];

//     let bindings = windows_bindgen::standalone(&apis);
//     std::fs::write("src/windows_term/bindings.rs", bindings).expect("failed to generate bindings");
// }
