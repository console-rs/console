use std::{
    borrow::Cow,
    iter::{FusedIterator, Peekable},
    ops::Range,
    str::CharIndices,
};

// TODO: make a diagram for this state machine to make it easier to get an overview
// TODO: can we combine the numeric stuff to reduce the number of states or does it have to be
// separate?
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
    S12,
    S13,
    S14,
    Trap,
}

impl Default for State {
    fn default() -> Self {
        Self::Start
    }
}

impl State {
    fn is_final(&self) -> bool {
        use State::*;

        matches!(
            self,
            S3 | S5 | S6 | S7 | S8 | S9 | S10 | S11 | S12 | S13 | S14
        )
    }

    fn is_trapped(&self) -> bool {
        matches!(self, Self::Trap)
    }

    fn transition(&mut self, c: char) {
        use State::*;

        let prev_state = *self;
        *self = match (prev_state, c) {
            (Start, '\u{1b}' | '\u{9b}') => S1,
            (S1, '(' | ')') => S2,
            (S1, '[' | '#' | ';' | '?') => S4,
            (S1 | S4, '0'..='9') => S5,
            (
                S1 | S4 | S5 | S6 | S7 | S8 | S9 | S10 | S11 | S12 | S13,
                'A'..='P' | 'R' | 'Z' | 'c' | 'f'..='n' | 'q' | 'r' | 'y' | '=' | '>' | '<',
            ) => S14,
            (S2, '0'..='2' | 'A' | 'B') => S3,
            (S2 | S4, '[' | '(' | ')' | '#' | ';' | '?') => S4,
            (S5, '0'..='9') => S6,
            (S5 | S6 | S7 | S8 | S9 | S10 | S11 | S12 | S13, ';') => S9,
            (S6, '0'..='9') => S7,
            (S7, '0'..='9') => S8,
            (S9, '0'..='9') => S10,
            (S10, '0'..='9') => S11,
            (S11, '0'..='9') => S12,
            (S12, '0'..='9') => S13,
            _ => Trap,
        }
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

// This purposfully mimics regex's `Match`
#[derive(Debug)]
struct Match<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

impl<'a> Match<'a> {
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    #[inline]
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    #[inline]
    pub fn as_str(&self) -> &'a str {
        &self.text[self.range()]
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

fn find_ansi_code_exclusive<'a>(it: &mut Peekable<CharIndices<'a>>) -> Option<(usize, usize)> {
    loop {
        if let (start, '\u{1b}' | '\u{9b}') = it.peek()? {
            let start = *start;
            let mut state = State::default();
            let mut maybe_end = None;

            loop {
                // TODO: store the peeked value and then special case some actions on being none
                // and then end based off it being `None` or it being trapped
                match it.peek() {
                    Some((idx, c)) => {
                        state.transition(*c);

                        // The match is greedy so run till we hit the trap state no matter what. A valid
                        // match is just one that was final at some point
                        if state.is_final() {
                            maybe_end = Some(*idx);
                        } else if state.is_trapped() {
                            match maybe_end {
                                Some(end) => {
                                    // All possible final characters are a single byte so it's safe to make
                                    // the end exclusive by just adding one
                                    return Some((start, end + 1));
                                }
                                None => break,
                            }
                        }

                        it.next();
                    }
                    // Can this be deduped from the above
                    None => match maybe_end {
                        Some(end) => {
                            return Some((start, end + 1));
                        }
                        None => break,
                    },
                }
            }
        }

        it.next();
    }
}

/// Helper function to strip ansi codes.
pub fn strip_ansi_codes(s: &str) -> Cow<str> {
    // TODO: we can create the iterator and peek to see if it's `Borrowed`. If not then things will
    // be a bit more expensive
    todo!()
    // match find_ansi_code(s) {
    //     Some(_) => todo!(),
    //     None => Cow::Borrowed(s),
    // }
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
            let s = &self.s[self.last_idx..m.start()];
            self.last_idx = m.end();
            if s.is_empty() {
                self.cur_idx = m.end();
                Some((m.as_str(), true))
            } else {
                self.cur_idx = m.start();
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

// TODO: add a proptest test to make sure that our `Matches` implementation matches the regex
#[cfg(test)]
mod tests {
    use super::*;

    mod old_regex_based {
        use std::borrow::Cow;

        use once_cell::sync::Lazy;
        use regex::Regex;

        pub static STRIP_ANSI_RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
        r"[\x1b\x9b]([()][012AB]|[\[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-PRZcf-nqry=><])",
            )
            .unwrap()
        });

        pub fn strip_ansi_codes(s: &str) -> Cow<str> {
            STRIP_ANSI_RE.replace_all(s, "")
        }
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
    fn blah() {
        use crate::style;
        // let s = &format!("{}", style("a").red().bold().force_styling(true));
        let s = "a\x1b[0m";
        let mut it = s.char_indices().peekable();
        panic!("{:?}", find_ansi_code_exclusive(&mut it));
        let new_matches: Vec<_> = Matches::new(s).collect();
        let old_matches: Vec<_> = old_regex_based::STRIP_ANSI_RE.find_iter(s).collect();
        println!("{:#?}", new_matches);
        println!("{:#?}", old_matches);

        panic!();
    }
}
