/// Key mapping
///
/// This is an incomplete mapping of keys that are supported for reading
/// from the keyboard.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum Key {
    Unknown,
    /// Unrecognized sequence containing Esc and a list of chars
    UnknownEscSeq(Vec<char>),
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
    Alt,
    Del,
    Shift,
    Insert,
    PageUp,
    PageDown,
    Char(char),
}

/// Converts a slice of `Key` enum values to a UTF-8 encoded `String`.
///Will add newlines for Key::Enter and delete the last char for Key::BackSpace
///
/// # Arguments
///
/// * `keys` - A slice of `Key` enum values representing user input keys.
pub fn keys_to_utf8(keys: &[Key]) -> String {
    let mut chars = Vec::new();
    for key in keys {
        match key {
            Key::Char(c) => chars.push(c),
            Key::Backspace => {
                chars.pop();
            }
            #[cfg(not(windows))]
            Key::Enter => chars.push(&'\n'),
            #[cfg(windows)]
            Key::Enter => {
                chars.push(&'\r');
                chars.push(&'\n')
            }
            key => {
                // This may be expanded by keeping track of a cursor which is controlled by the ArrowKeys and changes del and backspace
                unimplemented!("Cannot convert key: {:?} to utf8", key)
            }
        }
    }
    chars.into_iter().collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keys_to_utf8() {
        let keys = vec![
            Key::Char('H'),
            Key::Char('e'),
            Key::Char('l'),
            Key::Char('l'),
            Key::Char('o'),
            Key::Enter,
            Key::Char('W'),
            Key::Char('o'),
            Key::Char('r'),
            Key::Char('l'),
            Key::Char('d'),
            Key::Backspace,
        ];
        let result = keys_to_utf8(&keys);
        assert_eq!(result, "Hello\nWorl");
    }
}
