use std::{
    borrow::Cow,
    iter::{FusedIterator, Peekable},
    str::CharIndices,
};
use std::str::FromStr;
use lazy_static::lazy_static;
use regex::Regex;
use crate::{Attribute, Color, Style, StyledObject};

#[derive(Debug, Clone, Copy)]
enum State {
    Start,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    Trap,
}

impl Default for State {
    fn default() -> Self {
        Self::Start
    }
}

impl State {
    fn is_final(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Self::S3 | Self::S5 | Self::S6 | Self::S7 | Self::S8 | Self::S9 | Self::S11 => true,
            _ => false,
        }
    }

    fn is_trapped(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Self::Trap => true,
            _ => false,
        }
    }

    fn transition(&mut self, c: char) {
        *self = match c {
            '\u{1b}' | '\u{9b}' => match self {
                Self::Start => Self::S1,
                _ => Self::Trap,
            },
            '(' | ')' => match self {
                Self::S1 => Self::S2,
                Self::S2 | Self::S4 => Self::S4,
                _ => Self::Trap,
            },
            ';' => match self {
                Self::S1 | Self::S2 | Self::S4 => Self::S4,
                Self::S5 | Self::S6 | Self::S7 | Self::S8 | Self::S10 => Self::S10,
                _ => Self::Trap,
            },

            '[' | '#' | '?' => match self {
                Self::S1 | Self::S2 | Self::S4 => Self::S4,
                _ => Self::Trap,
            },
            '0'..='2' => match self {
                Self::S1 | Self::S4 => Self::S5,
                Self::S2 => Self::S3,
                Self::S5 => Self::S6,
                Self::S6 => Self::S7,
                Self::S7 => Self::S8,
                Self::S8 => Self::S9,
                Self::S10 => Self::S5,
                _ => Self::Trap,
            },
            '3'..='9' => match self {
                Self::S1 | Self::S4 => Self::S5,
                Self::S2 => Self::S5,
                Self::S5 => Self::S6,
                Self::S6 => Self::S7,
                Self::S7 => Self::S8,
                Self::S8 => Self::S9,
                Self::S10 => Self::S5,
                _ => Self::Trap,
            },
            'A'..='P' | 'R' | 'Z' | 'c' | 'f'..='n' | 'q' | 'r' | 'y' | '=' | '>' | '<' => {
                match self {
                    Self::S1
                    | Self::S2
                    | Self::S4
                    | Self::S5
                    | Self::S6
                    | Self::S7
                    | Self::S8
                    | Self::S10 => Self::S11,
                    _ => Self::Trap,
                }
            }
            _ => Self::Trap,
        };
    }
}

#[derive(Debug)]
struct Matches<'a> {
    s: &'a str,
    it: Peekable<CharIndices<'a>>,
}

impl<'a> Matches<'a> {
    fn new(s: &'a str) -> Self {
        let it = s.char_indices().peekable();
        Self { s, it }
    }
}

#[derive(Debug)]
struct Match<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

impl<'a> Match<'a> {
    #[inline]
    pub fn as_str(&self) -> &'a str {
        &self.text[self.start..self.end]
    }
}

impl<'a> Iterator for Matches<'a> {
    type Item = Match<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        find_ansi_code_exclusive(&mut self.it).map(|(start, end)| Match {
            text: self.s,
            start,
            end,
        })
    }
}

impl<'a> FusedIterator for Matches<'a> {}

fn find_ansi_code_exclusive(it: &mut Peekable<CharIndices>) -> Option<(usize, usize)> {
    'outer: loop {
        if let (start, '\u{1b}') | (start, '\u{9b}') = it.peek()? {
            let start = *start;
            let mut state = State::default();
            let mut maybe_end = None;

            loop {
                let item = it.peek();

                if let Some((idx, c)) = item {
                    state.transition(*c);

                    if state.is_final() {
                        maybe_end = Some(*idx);
                    }
                }

                // The match is greedy so run till we hit the trap state no matter what. A valid
                // match is just one that was final at some point
                if state.is_trapped() || item.is_none() {
                    match maybe_end {
                        Some(end) => {
                            // All possible final characters are a single byte so it's safe to make
                            // the end exclusive by just adding one
                            return Some((start, end + 1));
                        }
                        // The character we are peeking right now might be the start of a match so
                        // we want to continue the loop without popping off that char
                        None => continue 'outer,
                    }
                }

                it.next();
            }
        }

        it.next();
    }
}

/// Helper function to strip ansi codes.
pub fn strip_ansi_codes(s: &str) -> Cow<str> {
    let mut char_it = s.char_indices().peekable();
    match find_ansi_code_exclusive(&mut char_it) {
        Some(_) => {
            let stripped: String = AnsiCodeIterator::new(s)
                .filter_map(|(text, is_ansi)| if is_ansi { None } else { Some(text) })
                .collect();
            Cow::Owned(stripped)
        }
        None => Cow::Borrowed(s),
    }
}

/// An iterator over ansi codes in a string.
///
/// This type can be used to scan over ansi codes in a string.
/// It yields tuples in the form `(s, is_ansi)` where `s` is a slice of
/// the original string and `is_ansi` indicates if the slice contains
/// ansi codes or string values.
pub struct AnsiCodeIterator<'a> {
    s: &'a str,
    pending_item: Option<(&'a str, bool)>,
    last_idx: usize,
    cur_idx: usize,
    iter: Matches<'a>,
}

impl<'a> AnsiCodeIterator<'a> {
    /// Creates a new ansi code iterator.
    pub fn new(s: &'a str) -> AnsiCodeIterator<'a> {
        AnsiCodeIterator {
            s,
            pending_item: None,
            last_idx: 0,
            cur_idx: 0,
            iter: Matches::new(s),
        }
    }

    /// Returns the string slice up to the current match.
    pub fn current_slice(&self) -> &str {
        &self.s[..self.cur_idx]
    }

    /// Returns the string slice from the current match to the end.
    pub fn rest_slice(&self) -> &str {
        &self.s[self.cur_idx..]
    }
}

impl<'a> Iterator for AnsiCodeIterator<'a> {
    type Item = (&'a str, bool);

    fn next(&mut self) -> Option<(&'a str, bool)> {
        if let Some(pending_item) = self.pending_item.take() {
            self.cur_idx += pending_item.0.len();
            Some(pending_item)
        } else if let Some(m) = self.iter.next() {
            let s = &self.s[self.last_idx..m.start];
            self.last_idx = m.end;
            if s.is_empty() {
                self.cur_idx = m.end;
                Some((m.as_str(), true))
            } else {
                self.cur_idx = m.start;
                self.pending_item = Some((m.as_str(), true));
                Some((s, false))
            }
        } else if self.last_idx < self.s.len() {
            let rv = &self.s[self.last_idx..];
            self.cur_idx = self.s.len();
            self.last_idx = self.s.len();
            Some((rv, false))
        } else {
            None
        }
    }
}

impl<'a> FusedIterator for AnsiCodeIterator<'a> {}

/// An iterator over styled objects in a string.
///
/// This type can be used to scan over styled objects in a string.
pub struct ParsedStyledObjectIterator<'a> {
    ansi_code_it: AnsiCodeIterator<'a>,
}

impl<'a> ParsedStyledObjectIterator<'a> {
    pub fn new(s: &'a str) -> ParsedStyledObjectIterator<'a> {
        ParsedStyledObjectIterator {
            ansi_code_it: AnsiCodeIterator::new(s),
        }
    }

    /// parse a ansi code string to u8
    fn parse_ansi_num(ansi_str: &str) -> Option<u8> {
        let number = Regex::new("[1-9]\\d?m").unwrap();
        // find first str which matched xxm, such as 1m, 2m, 31m
        number.find(ansi_str).map(|r| {
            let r_str = r.as_str();
            // trim the 'm' and convert to u8
            u8::from_str(&r_str[0..r_str.len() - 1]).unwrap()
        })
    }

    /// convert ansi_num to color
    /// return (color: Color, bright: bool)
    fn convert_to_color(ansi_num: &u8) -> (Color, bool) {
        let mut bright = false;
        let ansi_num = if (40u8..47u8).contains(ansi_num) {
            ansi_num - 40
        } else if (30u8..37u8).contains(ansi_num) {
            ansi_num - 30
        } else if (8u8..15u8).contains(ansi_num) {
            bright = true;
            ansi_num - 8
        } else {
            *ansi_num
        };
        match ansi_num {
            0 => (Color::Black, bright),
            1 => (Color::Red, bright),
            2 => (Color::Green, bright),
            3 => (Color::Yellow, bright),
            4 => (Color::Blue, bright),
            5 => (Color::Magenta, bright),
            6 => (Color::Cyan, bright),
            7 => (Color::White, bright),
            _ => (Color::Color256(ansi_num), bright),
        }
    }

    fn convert_to_attr(ansi_num: &u8) -> Option<Attribute> {
        match ansi_num {
            1 => Some(Attribute::Bold),
            2 => Some(Attribute::Dim),
            3 => Some(Attribute::Italic),
            4 => Some(Attribute::Underlined),
            5 => Some(Attribute::Blink),
            7 => Some(Attribute::Reverse),
            8 => Some(Attribute::Hidden),
            _ => None,
        }
    }
}

lazy_static! {
static ref FG_COLOR256_OR_BRIGHT_REG: Regex = Regex::new("\x1b\\[38;5;[1-9]\\d?m").unwrap();
static ref FG_COLOR_REG: Regex = Regex::new("\x1b\\[3\\dm").unwrap();

static ref BG_COLOR256_OR_BRIGHT_REG: Regex = Regex::new("\x1b\\[48;5;[1-9]\\d?m").unwrap();
static ref BG_COLOR_REG: Regex = Regex::new("\x1b\\[4\\dm").unwrap();

static ref ATTR_REG: Regex = Regex::new("\x1b\\[[1-9]m").unwrap();
}

static RESET_STR: &str = "\x1b[0m";

impl<'a> Iterator for ParsedStyledObjectIterator<'a> {
    type Item = (String, Option<Style>);

    fn next(&mut self) -> Option<Self::Item> {
        let mut style_option: Option<Style> = None;
        let mut val: String = "".to_string();

        let mut ansi_start = false;
        let mut has_next = false;

        for (ansi_str, is_ansi) in self.ansi_code_it.by_ref() {
            has_next = true;
            if !is_ansi {
                val.push_str(ansi_str);
                if !ansi_start {
                    break;
                }
                continue;
            }
            if ansi_str == RESET_STR {
                // if is_ansi == true and ansi_str is reset, it means that ansi code is end
                break;
            }
            // if is_ansi == true and ansi_str is not reset, it means that ansi code is start
            ansi_start = true;
            if FG_COLOR_REG.is_match(ansi_str) || FG_COLOR256_OR_BRIGHT_REG.is_match(ansi_str) {
                if let Some(n) = Self::parse_ansi_num(ansi_str) {
                    let (color, bright) = Self::convert_to_color(&n);
                    style_option = Some(style_option.unwrap_or(Style::new()).fg(color));
                    if bright {
                        style_option = Some(style_option.unwrap_or(Style::new()).bright());
                    }
                }
            } else if BG_COLOR_REG.is_match(ansi_str) || BG_COLOR256_OR_BRIGHT_REG.is_match(ansi_str) {
                if let Some(n) = Self::parse_ansi_num(ansi_str) {
                    let (color, bright) = Self::convert_to_color(&n);
                    style_option = Some(style_option.unwrap_or(Style::new()).bg(color));
                    if bright {
                        style_option = Some(style_option.unwrap_or(Style::new()).on_bright());
                    }
                }
            } else if ATTR_REG.is_match(ansi_str) {
                if let Some(n) = Self::parse_ansi_num(ansi_str) {
                    if let Some(attr) = Self::convert_to_attr(&n) {
                        style_option = Some(style_option.unwrap_or(Style::new()).attr(attr));
                    }
                }
            }
        }

        style_option = style_option.map(|so| so.force_styling(true));

        match has_next {
            false => None,
            true => Some((val, style_option)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use lazy_static::lazy_static;
    use proptest::prelude::*;
    use regex::Regex;

    // The manual dfa `State` is a handwritten translation from the previously used regex. That
    // regex is kept here and used to ensure that the new matches are the same as the old
    lazy_static! {
        static ref STRIP_ANSI_RE: Regex = Regex::new(
            r"[\x1b\x9b]([()][012AB]|[\[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-PRZcf-nqry=><])",
        )
        .unwrap();
    }

    impl<'a, 'b> PartialEq<Match<'a>> for regex::Match<'b> {
        fn eq(&self, other: &Match<'a>) -> bool {
            self.start() == other.start && self.end() == other.end
        }
    }

    proptest! {
        #[test]
        fn dfa_matches_old_regex(s in r"([\x1b\x9b]?.*){0,5}") {
            let old_matches: Vec<_> = STRIP_ANSI_RE.find_iter(&s).collect();
            let new_matches: Vec<_> = Matches::new(&s).collect();
            assert_eq!(old_matches, new_matches);
        }
    }

    #[test]
    fn dfa_matches_regex_on_small_strings() {
        // To make sure the test runs in a reasonable time this is a slimmed down list of
        // characters to reduce the groups that are only used with each other along with one
        // arbitrarily chosen character not used in the regex (' ')
        const POSSIBLE_BYTES: &[u8] = &[b' ', 0x1b, 0x9b, b'(', b'0', b'[', b';', b'3', b'C'];

        fn check_all_strings_of_len(len: usize) {
            _check_all_strings_of_len(len, &mut Vec::with_capacity(len));
        }

        fn _check_all_strings_of_len(len: usize, chunk: &mut Vec<u8>) {
            if len == 0 {
                if let Ok(s) = std::str::from_utf8(chunk) {
                    let old_matches: Vec<_> = STRIP_ANSI_RE.find_iter(s).collect();
                    let new_matches: Vec<_> = Matches::new(s).collect();
                    assert_eq!(old_matches, new_matches);
                }

                return;
            }

            for b in POSSIBLE_BYTES {
                chunk.push(*b);
                _check_all_strings_of_len(len - 1, chunk);
                chunk.pop();
            }
        }

        for str_len in 0..=6 {
            check_all_strings_of_len(str_len);
        }
    }

    #[test]
    fn complex_data() {
        let s = std::fs::read_to_string(
            std::path::Path::new("tests")
                .join("data")
                .join("sample_zellij_session.log"),
        )
        .unwrap();

        let old_matches: Vec<_> = STRIP_ANSI_RE.find_iter(&s).collect();
        let new_matches: Vec<_> = Matches::new(&s).collect();
        assert_eq!(old_matches, new_matches);
    }

    #[test]
    fn state_machine() {
        let ansi_code = "\x1b)B";
        let mut state = State::default();
        assert!(!state.is_final());

        for c in ansi_code.chars() {
            state.transition(c);
        }
        assert!(state.is_final());

        state.transition('A');
        assert!(state.is_trapped());
    }

    #[test]
    fn back_to_back_entry_char() {
        let s = "\x1b\x1bf";
        let matches: Vec<_> = Matches::new(s).map(|m| m.as_str()).collect();
        assert_eq!(&["\x1bf"], matches.as_slice());
    }

    #[test]
    fn early_paren_can_use_many_chars() {
        let s = "\x1b(C";
        let matches: Vec<_> = Matches::new(s).map(|m| m.as_str()).collect();
        assert_eq!(&[s], matches.as_slice());
    }

    #[test]
    fn long_run_of_digits() {
        let s = "\u{1b}00000";
        let matches: Vec<_> = Matches::new(s).map(|m| m.as_str()).collect();
        assert_eq!(&[s], matches.as_slice());
    }

    #[test]
    fn test_ansi_iter_re_vt100() {
        let s = "\x1b(0lpq\x1b)Benglish";
        let mut iter = AnsiCodeIterator::new(s);
        assert_eq!(iter.next(), Some(("\x1b(0", true)));
        assert_eq!(iter.next(), Some(("lpq", false)));
        assert_eq!(iter.next(), Some(("\x1b)B", true)));
        assert_eq!(iter.next(), Some(("english", false)));
    }

    #[test]
    fn test_ansi_iter_re() {
        use crate::style;
        let s = format!("Hello {}!", style("World").red().force_styling(true));
        let mut iter = AnsiCodeIterator::new(&s);
        assert_eq!(iter.next(), Some(("Hello ", false)));
        assert_eq!(iter.current_slice(), "Hello ");
        assert_eq!(iter.rest_slice(), "\x1b[31mWorld\x1b[0m!");
        assert_eq!(iter.next(), Some(("\x1b[31m", true)));
        assert_eq!(iter.current_slice(), "Hello \x1b[31m");
        assert_eq!(iter.rest_slice(), "World\x1b[0m!");
        assert_eq!(iter.next(), Some(("World", false)));
        assert_eq!(iter.current_slice(), "Hello \x1b[31mWorld");
        assert_eq!(iter.rest_slice(), "\x1b[0m!");
        assert_eq!(iter.next(), Some(("\x1b[0m", true)));
        assert_eq!(iter.current_slice(), "Hello \x1b[31mWorld\x1b[0m");
        assert_eq!(iter.rest_slice(), "!");
        assert_eq!(iter.next(), Some(("!", false)));
        assert_eq!(iter.current_slice(), "Hello \x1b[31mWorld\x1b[0m!");
        assert_eq!(iter.rest_slice(), "");
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_ansi_iter_re_on_multi() {
        use crate::style;
        let s = format!("{}", style("a").red().bold().force_styling(true));
        let mut iter = AnsiCodeIterator::new(&s);
        assert_eq!(iter.next(), Some(("\x1b[31m", true)));
        assert_eq!(iter.current_slice(), "\x1b[31m");
        assert_eq!(iter.rest_slice(), "\x1b[1ma\x1b[0m");
        assert_eq!(iter.next(), Some(("\x1b[1m", true)));
        assert_eq!(iter.current_slice(), "\x1b[31m\x1b[1m");
        assert_eq!(iter.rest_slice(), "a\x1b[0m");
        assert_eq!(iter.next(), Some(("a", false)));
        assert_eq!(iter.current_slice(), "\x1b[31m\x1b[1ma");
        assert_eq!(iter.rest_slice(), "\x1b[0m");
        assert_eq!(iter.next(), Some(("\x1b[0m", true)));
        assert_eq!(iter.current_slice(), "\x1b[31m\x1b[1ma\x1b[0m");
        assert_eq!(iter.rest_slice(), "");
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_parse_to_style_for_multi_text() {
        let style_origin1 = Style::new()
            .force_styling(true)
            .red()
            .on_blue()
            .on_bright()
            .bold()
            .italic();
        let style_origin2 = Style::new()
            .force_styling(true)
            .blue()
            .on_yellow()
            .on_bright()
            .blink()
            .italic();

        // test for "[hello world]"
        let ansi_string = style_origin1.apply_to("hello world").to_string();
        let style_parsed = ParsedStyledObjectIterator::new(ansi_string.as_str()).collect::<Vec<(String, Option<Style>)>>();
        let plain_texts = style_parsed.iter().map(|x| &x.0).collect::<Vec<_>>();
        let styles = style_parsed.iter().map(|x| x.1.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            vec!["hello world"],
            plain_texts,
        );
        assert_eq!(
            vec![Some(&style_origin1)],
            styles,
        );

        // test for "[hello] [world]"
        let ansi_string = format!("{} {}", style_origin1.apply_to("hello"), style_origin2.apply_to("world"));
        let style_parsed = ParsedStyledObjectIterator::new(ansi_string.as_str()).collect::<Vec<(String, Option<Style>)>>();
        let plain_texts = style_parsed.iter().map(|x| &x.0).collect::<Vec<_>>();
        let styles = style_parsed.iter().map(|x| x.1.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            vec!["hello", " ", "world"],
            plain_texts
        );
        assert_eq!(
            vec![Some(&style_origin1), None, Some(&style_origin2)],
            styles
        );

        // test for "hello [world]"
        let ansi_string = format!("hello {}", style_origin2.apply_to("world"));
        let style_parsed = ParsedStyledObjectIterator::new(ansi_string.as_str()).collect::<Vec<(String, Option<Style>)>>();
        let plain_texts = style_parsed.iter().map(|x| &x.0).collect::<Vec<_>>();
        let styles = style_parsed.iter().map(|x| x.1.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            vec!["hello ", "world"],
            plain_texts
        );
        assert_eq!(
            vec![None, Some(&style_origin2)],
            styles
        );

        // test for "[hello] world"
        let ansi_string = format!("{} world", style_origin1.apply_to("hello"));
        let style_parsed = ParsedStyledObjectIterator::new(ansi_string.as_str()).collect::<Vec<(String, Option<Style>)>>();
        let plain_texts = style_parsed.iter().map(|x| &x.0).collect::<Vec<_>>();
        let styles = style_parsed.iter().map(|x| x.1.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            vec!["hello", " world"],
            plain_texts
        );
        assert_eq!(
            vec![Some(&style_origin1), None],
            styles
        );

        // test for "hello world"
        let ansi_string = "hello world";
        let style_parsed = ParsedStyledObjectIterator::new(ansi_string).collect::<Vec<(String, Option<Style>)>>();
        let plain_texts = style_parsed.iter().map(|x| &x.0).collect::<Vec<_>>();
        let styles = style_parsed.iter().map(|x| x.1.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            vec!["hello world"],
            plain_texts
        );
        assert_eq!(
            vec![None],
            styles
        );
    }
}
