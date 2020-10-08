/// Key mapping
///
/// This is an incomplete mapping of keys that are supported for reading
/// from the keyboard.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Key {
    Unknown,
    // Escape followed by an unrecognized sequence
    UnknownEscSeq,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    Backspace,
    Home,
    End,
    Tab,
    BackTab,
    Del,
    Insert,
    PageUp,
    PageDown,
    Char(char),
    #[doc(hidden)]
    __More,
}
