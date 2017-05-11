//! console is a library for Rust that provides access to various terminal
//! features so you can build nicer looking command line interfaces.  It
//! comes with various tools and utilities for working with Terminals and
//! formatting text.
//!
//! Best paired with other libraries in the family:
//!
//! * [dialoguer](https://crates.io/crates/dialoguer)
//! * [indicatif](https://crates.io/crates/indicatif)
//!
//! # Terminal Access
//!
//! The terminal is abstracted through the `console::Term` type.  It can
//! either directly provide access to the connected terminal or by buffering
//! up commands.  A buffered terminal will however not be completely buffered
//! on windows where cursor movements are currently directly passed through.
//!
//! Example usage:
//!
//! ```
//! # fn test() -> Result<(), Box<std::error::Error>> {
//! use std::thread;
//! use std::time::Duration;
//!
//! use console::Term;
//!
//! let term = Term::stdout();
//! term.write_line("Hello World!")?;
//! thread::sleep(Duration::from_millis(2000));
//! term.clear_line()?;
//! # Ok(()) } fn main() { test().unwrap(); }
//! ```
//!
//! # Colors and Styles
//!
//! `console` uses `clicolors-control` to control colors.  It also
//! provides higher level wrappers for styling text and other things
//! that can be displayed with the `style` function and utility types.
//!
//! Example usage:
//!
//! ```
//! use console::style;
//!
//! println!("This is {} neat", style("quite").cyan());
//! ```
//!
//! You can also store styles and apply them to text later:
//!
//! ```
//! use console::Style;
//!
//! let cyan = Style::new().cyan();
//! println!("This is {} neat", cyan.apply_to("quite"));
//! ```
//!
//! # Working with ANSI Codes
//!
//! The crate provids the function `strip_ansi_codes` to remove ANSI codes
//! from a string as well as `measure_text_width` to calculate the width of a
//! string as it would be displayed by the terminal.  Both of those together
//! are useful for more complex formatting.
#[cfg(unix)] extern crate libc;
#[cfg(unix)] extern crate termios;
#[cfg(windows)] extern crate winapi;
#[cfg(windows)] extern crate kernel32;
#[macro_use] extern crate lazy_static;
extern crate regex;
extern crate parking_lot;
extern crate unicode_width;
extern crate clicolors_control;

pub use kb::Key;
pub use term::{Term, user_attended};
pub use utils::{style, Style, StyledObject, Color, Attribute,
                Emoji, strip_ansi_codes, measure_text_width,
                colors_enabled, set_colors_enabled};

mod term;
mod utils;
mod kb;
#[cfg(unix)] mod unix_term;
#[cfg(windows)] mod windows_term;
