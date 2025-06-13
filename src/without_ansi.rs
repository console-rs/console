use core::fmt::Display;

use crate::AnsiCodeIterator;

/// A wrapper struct that implements [`core::fmt::Display`], only displaying non-ansi parts.
pub struct WithoutAnsi<'a> {
    str: &'a str,
}
impl<'a> WithoutAnsi<'a> {
    pub fn new(str: &'a str) -> Self {
        Self { str }
    }
}
impl Display for WithoutAnsi<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for str in
            AnsiCodeIterator::new(self.str)
                .filter_map(|(str, is_ansi)| if is_ansi { None } else { Some(str) })
        {
            f.write_str(str)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::fmt::Write;

    #[test]
    fn display_multiple_times() {
        let str_with_ansi = "\x1b[1;97;41mError\x1b[0m";
        let without_ansi = WithoutAnsi::new(str_with_ansi);
        for _ in 0..2 {
            let mut output = String::default();
            write!(output, "{without_ansi}").unwrap();
            assert_eq!(output, "Error");
        }
    }
}
