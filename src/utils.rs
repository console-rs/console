use alloc::borrow::Cow;
use core::{
    fmt::{self, Debug, Formatter},
    sync::atomic::{AtomicBool, Ordering},
};
use std::env;

use std::sync::OnceLock;

use crate::term::{wants_emoji, Term};

#[cfg(feature = "ansi-parsing")]
use crate::ansi::AnsiCodeIterator;

fn default_colors_enabled(out: &Term) -> bool {
    (out.features().colors_supported()
        && &env::var("CLICOLOR").unwrap_or_else(|_| "1".into()) != "0")
        || &env::var("CLICOLOR_FORCE").unwrap_or_else(|_| "0".into()) != "0"
}

fn default_true_colors_enabled(out: &Term) -> bool {
    out.features().true_colors_supported()
}

fn stdout_colors() -> &'static AtomicBool {
    static ENABLED: OnceLock<AtomicBool> = OnceLock::new();
    ENABLED.get_or_init(|| AtomicBool::new(default_colors_enabled(&Term::stdout())))
}
fn stdout_true_colors() -> &'static AtomicBool {
    static ENABLED: OnceLock<AtomicBool> = OnceLock::new();
    ENABLED.get_or_init(|| AtomicBool::new(default_true_colors_enabled(&Term::stdout())))
}
fn stderr_colors() -> &'static AtomicBool {
    static ENABLED: OnceLock<AtomicBool> = OnceLock::new();
    ENABLED.get_or_init(|| AtomicBool::new(default_colors_enabled(&Term::stderr())))
}
fn stderr_true_colors() -> &'static AtomicBool {
    static ENABLED: OnceLock<AtomicBool> = OnceLock::new();
    ENABLED.get_or_init(|| AtomicBool::new(default_true_colors_enabled(&Term::stderr())))
}

/// Returns `true` if colors should be enabled for stdout.
///
/// This honors the [clicolors spec](http://bixense.com/clicolors/).
///
/// * `CLICOLOR != 0`: ANSI colors are supported and should be used when the program isn't piped.
/// * `CLICOLOR == 0`: Don't output ANSI color escape codes.
/// * `CLICOLOR_FORCE != 0`: ANSI colors should be enabled no matter what.
#[inline]
pub fn colors_enabled() -> bool {
    stdout_colors().load(Ordering::Relaxed)
}

/// Returns `true` if true colors should be enabled for stdout.
#[inline]
pub fn true_colors_enabled() -> bool {
    stdout_true_colors().load(Ordering::Relaxed)
}

/// Forces colorization on or off for stdout.
///
/// This overrides the default for the current process and changes the return value of the
/// `colors_enabled` function.
#[inline]
pub fn set_colors_enabled(val: bool) {
    stdout_colors().store(val, Ordering::Relaxed)
}

/// Forces true colorization on or off for stdout.
///
/// This overrides the default for the current process and changes the return value of the
/// `true_colors_enabled` function.
#[inline]
pub fn set_true_colors_enabled(val: bool) {
    stdout_true_colors().store(val, Ordering::Relaxed)
}

/// Returns `true` if colors should be enabled for stderr.
///
/// This honors the [clicolors spec](http://bixense.com/clicolors/).
///
/// * `CLICOLOR != 0`: ANSI colors are supported and should be used when the program isn't piped.
/// * `CLICOLOR == 0`: Don't output ANSI color escape codes.
/// * `CLICOLOR_FORCE != 0`: ANSI colors should be enabled no matter what.
#[inline]
pub fn colors_enabled_stderr() -> bool {
    stderr_colors().load(Ordering::Relaxed)
}

/// Returns `true` if true colors should be enabled for stderr.
#[inline]
pub fn true_colors_enabled_stderr() -> bool {
    stderr_true_colors().load(Ordering::Relaxed)
}

/// Forces colorization on or off for stderr.
///
/// This overrides the default for the current process and changes the return value of the
/// `colors_enabled_stderr` function.
#[inline]
pub fn set_colors_enabled_stderr(val: bool) {
    stderr_colors().store(val, Ordering::Relaxed)
}

/// Forces true colorization on or off for stderr.
///
/// This overrides the default for the current process and changes the return value of the
/// `true_colors_enabled_stderr` function.
#[inline]
pub fn set_true_colors_enabled_stderr(val: bool) {
    stderr_true_colors().store(val, Ordering::Relaxed)
}

/// Measure the width of a string in terminal characters.
pub fn measure_text_width(s: &str) -> usize {
    #[cfg(feature = "ansi-parsing")]
    {
        AnsiCodeIterator::new(s)
            .filter_map(|(s, is_ansi)| match is_ansi {
                false => Some(str_width(s)),
                true => None,
            })
            .sum()
    }
    #[cfg(not(feature = "ansi-parsing"))]
    {
        str_width(s)
    }
}

/// A terminal color.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Color256(u8),
    TrueColor(u8, u8, u8),
}

impl Color {
    #[inline]
    fn ansi_num(self) -> usize {
        match self {
            Color::Black => 0,
            Color::Red => 1,
            Color::Green => 2,
            Color::Yellow => 3,
            Color::Blue => 4,
            Color::Magenta => 5,
            Color::Cyan => 6,
            Color::White => 7,
            Color::Color256(x) => x as usize,
            Color::TrueColor(_, _, _) => panic!("RGB colors must be handled separately"),
        }
    }

    #[inline]
    fn is_color256(self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Color::Color256(_) => true,
            _ => false,
        }
    }

    /// Converts a color to its RGB representation.
    ///
    /// Basic ANSI colors are converted to their standard RGB equivalents.
    /// Color256 values are converted using the standard 256-color palette.
    #[inline]
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            Color::Black => (0, 0, 0),
            Color::Red => (205, 49, 49),
            Color::Green => (13, 188, 121),
            Color::Yellow => (229, 229, 16),
            Color::Blue => (36, 114, 200),
            Color::Magenta => (188, 63, 188),
            Color::Cyan => (17, 168, 205),
            Color::White => (229, 229, 229),
            Color::Color256(n) => color256_to_rgb(*n),
            Color::TrueColor(r, g, b) => (*r, *g, *b),
        }
    }

    /// Interpolates between two colors at a given position.
    ///
    /// The position `t` should be between 0.0 and 1.0, where 0.0 returns
    /// `self` and 1.0 returns `other`.
    #[inline]
    pub fn interpolate(self, other: Self, t: f32) -> (u8, u8, u8) {
        let (r1, g1, b1) = self.to_rgb();
        let (r2, g2, b2) = other.to_rgb();

        let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t).round() as u8;
        let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t).round() as u8;
        let b = (b1 as f32 + (b2 as f32 - b1 as f32) * t).round() as u8;

        (r, g, b)
    }
}

/// Parses a hex color string like "#ff0000" into a (r, g, b) tuple.
fn parse_hex_color(s: &str) -> Option<(u8, u8, u8)> {
    if s.len() != 7 || !s.starts_with('#') {
        return None;
    }

    let r = u8::from_str_radix(&s[1..3], 16).ok()?;
    let g = u8::from_str_radix(&s[3..5], 16).ok()?;
    let b = u8::from_str_radix(&s[5..7], 16).ok()?;

    Some((r, g, b))
}

/// Converts a 256-color palette index to RGB values.
fn color256_to_rgb(n: u8) -> (u8, u8, u8) {
    match n {
        0 => (0, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        // 216 color cube (16-231)
        16..=231 => {
            let n = n - 16;
            let b = n % 6;
            let g = (n / 6) % 6;
            let r = n / 36;

            let to_val = |x: u8| if x == 0 { 0 } else { 55 + x * 40 };

            (to_val(r), to_val(g), to_val(b))
        }
        // Grayscale (232-255)
        232..=255 => {
            let v = 8 + (n - 232) * 10;

            (v, v, v)
        }
    }
}

/// A terminal style attribute.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
#[repr(u16)]
pub enum Attribute {
    // This mapping is important, it exactly matches ansi_num = (x as u16 + 1)
    // See `ATTRIBUTES_LOOKUP` as well
    Bold = 0,
    Dim = 1,
    Italic = 2,
    Underlined = 3,
    Blink = 4,
    BlinkFast = 5,
    Reverse = 6,
    Hidden = 7,
    StrikeThrough = 8,
}

impl Attribute {
    const MAP: [Attribute; 9] = [
        Attribute::Bold,
        Attribute::Dim,
        Attribute::Italic,
        Attribute::Underlined,
        Attribute::Blink,
        Attribute::BlinkFast,
        Attribute::Reverse,
        Attribute::Hidden,
        Attribute::StrikeThrough,
    ];
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Attributes(u16);

impl Attributes {
    #[inline]
    const fn new() -> Self {
        Self(0)
    }

    #[inline]
    #[must_use]
    const fn insert(mut self, attr: Attribute) -> Self {
        let bit = attr as u16;
        self.0 |= 1 << bit;
        self
    }

    #[inline]
    const fn bits(self) -> BitsIter {
        BitsIter(self.0)
    }

    #[inline]
    fn attrs(self) -> impl Iterator<Item = Attribute> {
        self.bits().map(|bit| Attribute::MAP[bit as usize])
    }

    #[inline]
    fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Attributes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for ansi in self.bits().map(|bit| bit + 1) {
            write!(f, "\x1b[{ansi}m")?;
        }
        Ok(())
    }
}

struct BitsIter(u16);

impl Iterator for BitsIter {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }
        let bit = self.0.trailing_zeros();
        self.0 ^= (1 << bit) as u16;
        Some(bit as u16)
    }
}

impl Debug for Attributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.attrs()).finish()
    }
}

/// Defines the alignment for padding operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

/// A stored style that can be applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    fg: Option<Color>,
    fg_end: Option<Color>,
    bg: Option<Color>,
    bg_end: Option<Color>,
    fg_bright: bool,
    bg_bright: bool,
    attrs: Attributes,
    force: Option<bool>,
    for_stderr: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

impl Style {
    /// Returns an empty default style.
    pub const fn new() -> Self {
        Self {
            fg: None,
            fg_end: None,
            bg: None,
            bg_end: None,
            fg_bright: false,
            bg_bright: false,
            attrs: Attributes::new(),
            force: None,
            for_stderr: false,
        }
    }

    /// Creates a style from a dotted string.
    ///
    /// Effectively the string is split at each dot and then the
    /// terms in between are applied.  For instance `red.on_blue` will
    /// create a string that is red on blue background. `9.on_12` is
    /// the same, but using 256 color numbers. Unknown terms are
    /// ignored.
    ///
    /// Gradients can be specified using `->` syntax:
    ///
    /// - `#ff0000->#00ff00` - foreground gradient from red to green
    /// - `on_#ff0000->#0000ff` - background gradient from red to blue
    pub fn from_dotted_str(s: &str) -> Self {
        let mut rv = Self::new();
        for part in s.split('.') {
            rv = match part {
                "black" => rv.black(),
                "red" => rv.red(),
                "green" => rv.green(),
                "yellow" => rv.yellow(),
                "blue" => rv.blue(),
                "magenta" => rv.magenta(),
                "cyan" => rv.cyan(),
                "white" => rv.white(),
                "bright" => rv.bright(),
                "on_black" => rv.on_black(),
                "on_red" => rv.on_red(),
                "on_green" => rv.on_green(),
                "on_yellow" => rv.on_yellow(),
                "on_blue" => rv.on_blue(),
                "on_magenta" => rv.on_magenta(),
                "on_cyan" => rv.on_cyan(),
                "on_white" => rv.on_white(),
                "on_bright" => rv.on_bright(),
                "bold" => rv.bold(),
                "dim" => rv.dim(),
                "underlined" => rv.underlined(),
                "blink" => rv.blink(),
                "blink_fast" => rv.blink_fast(),
                "reverse" => rv.reverse(),
                "hidden" => rv.hidden(),
                "strikethrough" => rv.strikethrough(),
                // Background gradient: on_#rrggbb->#rrggbb
                on_gradient if on_gradient.starts_with("on_#") && on_gradient.contains("->") => {
                    let Some((start, end)) = on_gradient[3..].split_once("->") else {
                        continue;
                    };

                    let Some((start, end)) = parse_hex_color(start).zip(parse_hex_color(end))
                    else {
                        continue;
                    };

                    rv.on_true_color(start.0, start.1, start.2)
                        .on_true_color_end(end.0, end.1, end.2)
                }
                // Foreground gradient: #rrggbb->#rrggbb
                gradient if gradient.starts_with('#') && gradient.contains("->") => {
                    let Some((start, end)) = gradient.split_once("->") else {
                        continue;
                    };

                    let Some((start, end)) = parse_hex_color(start).zip(parse_hex_color(end))
                    else {
                        continue;
                    };

                    rv.true_color(start.0, start.1, start.2)
                        .true_color_end(end.0, end.1, end.2)
                }
                on_true_color if on_true_color.starts_with("on_#") && on_true_color.len() == 10 => {
                    let Some(color) = parse_hex_color(&on_true_color[3..]) else {
                        continue;
                    };

                    rv.on_true_color(color.0, color.1, color.2)
                }
                true_color if true_color.starts_with('#') && true_color.len() == 7 => {
                    let Some(color) = parse_hex_color(true_color) else {
                        continue;
                    };

                    rv.true_color(color.0, color.1, color.2)
                }
                on_c if on_c.starts_with("on_") => {
                    if let Ok(n) = on_c[3..].parse::<u8>() {
                        rv.on_color256(n)
                    } else {
                        continue;
                    }
                }
                c => {
                    if let Ok(n) = c.parse::<u8>() {
                        rv.color256(n)
                    } else {
                        continue;
                    }
                }
            };
        }
        rv
    }

    /// Apply the style to something that can be displayed.
    pub fn apply_to<D>(&self, val: D) -> StyledObject<D> {
        StyledObject {
            style: *self,
            val,
            size_hint: None,
            position_hint: None,
        }
    }

    /// Forces styling on or off.
    ///
    /// This overrides the automatic detection.
    #[inline]
    pub const fn force_styling(mut self, value: bool) -> Self {
        self.force = Some(value);
        self
    }

    /// Specifies that style is applying to something being written on stderr.
    #[inline]
    pub const fn for_stderr(mut self) -> Self {
        self.for_stderr = true;
        self
    }

    /// Specifies that style is applying to something being written on stdout.
    ///
    /// This is the default behaviour.
    #[inline]
    pub const fn for_stdout(mut self) -> Self {
        self.for_stderr = false;
        self
    }

    /// Sets a foreground color.
    #[inline]
    pub const fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// Sets the foreground gradient end color.
    ///
    /// When a end color is set, the foreground will transition
    /// from the foreground color to this color across the text.
    ///
    /// Requires true color support in the terminal.
    #[inline]
    pub const fn fg_end(mut self, color: Color) -> Self {
        self.fg_end = Some(color);
        self
    }

    /// Sets a background color.
    #[inline]
    pub const fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Sets the background gradient end color.
    ///
    /// When a end color is set, the background will transition
    /// from the background color to this color across the background.
    ///
    /// Requires true color support in the terminal.
    #[inline]
    pub const fn bg_end(mut self, color: Color) -> Self {
        self.bg_end = Some(color);
        self
    }

    /// Adds a attr.
    #[inline]
    pub const fn attr(mut self, attr: Attribute) -> Self {
        self.attrs = self.attrs.insert(attr);
        self
    }

    #[inline]
    pub const fn black(self) -> Self {
        self.fg(Color::Black)
    }
    #[inline]
    pub const fn black_end(self) -> Self {
        self.fg_end(Color::Black)
    }
    #[inline]
    pub const fn red(self) -> Self {
        self.fg(Color::Red)
    }
    #[inline]
    pub const fn red_end(self) -> Self {
        self.fg_end(Color::Red)
    }
    #[inline]
    pub const fn green(self) -> Self {
        self.fg(Color::Green)
    }
    #[inline]
    pub const fn green_end(self) -> Self {
        self.fg_end(Color::Green)
    }
    #[inline]
    pub const fn yellow(self) -> Self {
        self.fg(Color::Yellow)
    }
    #[inline]
    pub const fn yellow_end(self) -> Self {
        self.fg_end(Color::Yellow)
    }
    #[inline]
    pub const fn blue(self) -> Self {
        self.fg(Color::Blue)
    }
    #[inline]
    pub const fn blue_end(self) -> Self {
        self.fg_end(Color::Blue)
    }
    #[inline]
    pub const fn magenta(self) -> Self {
        self.fg(Color::Magenta)
    }
    #[inline]
    pub const fn magenta_end(self) -> Self {
        self.fg_end(Color::Magenta)
    }
    #[inline]
    pub const fn cyan(self) -> Self {
        self.fg(Color::Cyan)
    }
    #[inline]
    pub const fn cyan_end(self) -> Self {
        self.fg_end(Color::Cyan)
    }
    #[inline]
    pub const fn white(self) -> Self {
        self.fg(Color::White)
    }
    #[inline]
    pub const fn white_end(self) -> Self {
        self.fg_end(Color::White)
    }
    #[inline]
    pub const fn color256(self, color: u8) -> Self {
        self.fg(Color::Color256(color))
    }
    #[inline]
    pub const fn color256_end(self, color: u8) -> Self {
        self.fg_end(Color::Color256(color))
    }
    #[inline]
    pub const fn true_color(self, r: u8, g: u8, b: u8) -> Self {
        self.fg(Color::TrueColor(r, g, b))
    }
    #[inline]
    pub const fn true_color_end(self, r: u8, g: u8, b: u8) -> Self {
        self.fg_end(Color::TrueColor(r, g, b))
    }

    #[inline]
    pub const fn bright(mut self) -> Self {
        self.fg_bright = true;
        self
    }

    #[inline]
    pub const fn on_black(self) -> Self {
        self.bg(Color::Black)
    }
    #[inline]
    pub const fn on_black_end(self) -> Self {
        self.bg_end(Color::Black)
    }
    #[inline]
    pub const fn on_red(self) -> Self {
        self.bg(Color::Red)
    }
    #[inline]
    pub const fn on_red_end(self) -> Self {
        self.bg_end(Color::Red)
    }
    #[inline]
    pub const fn on_green(self) -> Self {
        self.bg(Color::Green)
    }
    #[inline]
    pub const fn on_green_end(self) -> Self {
        self.bg_end(Color::Green)
    }
    #[inline]
    pub const fn on_yellow(self) -> Self {
        self.bg(Color::Yellow)
    }
    #[inline]
    pub const fn on_yellow_end(self) -> Self {
        self.bg_end(Color::Yellow)
    }
    #[inline]
    pub const fn on_blue(self) -> Self {
        self.bg(Color::Blue)
    }
    #[inline]
    pub const fn on_blue_end(self) -> Self {
        self.bg_end(Color::Blue)
    }
    #[inline]
    pub const fn on_magenta(self) -> Self {
        self.bg(Color::Magenta)
    }
    #[inline]
    pub const fn on_magenta_end(self) -> Self {
        self.bg_end(Color::Magenta)
    }
    #[inline]
    pub const fn on_cyan(self) -> Self {
        self.bg(Color::Cyan)
    }
    #[inline]
    pub const fn on_cyan_end(self) -> Self {
        self.bg_end(Color::Cyan)
    }
    #[inline]
    pub const fn on_white(self) -> Self {
        self.bg(Color::White)
    }
    #[inline]
    pub const fn on_white_end(self) -> Self {
        self.bg_end(Color::White)
    }
    #[inline]
    pub const fn on_color256(self, color: u8) -> Self {
        self.bg(Color::Color256(color))
    }
    #[inline]
    pub const fn on_color256_end(self, color: u8) -> Self {
        self.bg_end(Color::Color256(color))
    }
    #[inline]
    pub const fn on_true_color(self, r: u8, g: u8, b: u8) -> Self {
        self.bg(Color::TrueColor(r, g, b))
    }
    #[inline]
    pub const fn on_true_color_end(self, r: u8, g: u8, b: u8) -> Self {
        self.bg_end(Color::TrueColor(r, g, b))
    }

    #[inline]
    pub const fn on_bright(mut self) -> Self {
        self.bg_bright = true;
        self
    }

    #[inline]
    pub const fn bold(self) -> Self {
        self.attr(Attribute::Bold)
    }
    #[inline]
    pub const fn dim(self) -> Self {
        self.attr(Attribute::Dim)
    }
    #[inline]
    pub const fn italic(self) -> Self {
        self.attr(Attribute::Italic)
    }
    #[inline]
    pub const fn underlined(self) -> Self {
        self.attr(Attribute::Underlined)
    }
    #[inline]
    pub const fn blink(self) -> Self {
        self.attr(Attribute::Blink)
    }
    #[inline]
    pub const fn blink_fast(self) -> Self {
        self.attr(Attribute::BlinkFast)
    }
    #[inline]
    pub const fn reverse(self) -> Self {
        self.attr(Attribute::Reverse)
    }
    #[inline]
    pub const fn hidden(self) -> Self {
        self.attr(Attribute::Hidden)
    }
    #[inline]
    pub const fn strikethrough(self) -> Self {
        self.attr(Attribute::StrikeThrough)
    }

    #[inline]
    const fn get_fg_gradient(&self) -> Option<FgGradient> {
        let Some(start) = self.fg else {
            return None;
        };
        let Some(end) = self.fg_end else {
            return None;
        };

        Some(FgGradient(Gradient { start, end }))
    }

    #[inline]
    const fn get_bg_gradient(&self) -> Option<BgGradient> {
        let Some(start) = self.bg else {
            return None;
        };
        let Some(end) = self.bg_end else {
            return None;
        };

        Some(BgGradient(Gradient { start, end }))
    }
}

/// Wraps an object for formatting for styling.
///
/// Example:
///
/// ```rust,no_run
/// # use console::style;
/// format!("Hello {}", style("World").cyan());
/// ```
///
/// This is a shortcut for making a new style and applying it
/// to a value:
///
/// ```rust,no_run
/// # use console::Style;
/// format!("Hello {}", Style::new().cyan().apply_to("World"));
/// ```
pub fn style<D>(val: D) -> StyledObject<D> {
    Style::new().apply_to(val)
}

/// A formatting wrapper that can be styled for a terminal.
#[derive(Clone)]
pub struct StyledObject<D> {
    style: Style,
    val: D,
    size_hint: Option<usize>,
    position_hint: Option<usize>,
}

impl<D> StyledObject<D> {
    /// Forces styling on or off.
    ///
    /// This overrides the automatic detection.
    #[inline]
    pub fn force_styling(mut self, value: bool) -> StyledObject<D> {
        self.style = self.style.force_styling(value);
        self
    }

    /// Specifies that style is applying to something being written on stderr
    #[inline]
    pub fn for_stderr(mut self) -> StyledObject<D> {
        self.style = self.style.for_stderr();
        self
    }

    /// Specifies that style is applying to something being written on stdout
    ///
    /// This is the default
    #[inline]
    pub const fn for_stdout(mut self) -> StyledObject<D> {
        self.style = self.style.for_stdout();
        self
    }

    /// Sets a foreground color.
    #[inline]
    pub const fn fg(mut self, color: Color) -> StyledObject<D> {
        self.style = self.style.fg(color);
        self
    }

    /// Sets the foreground gradient end color.
    ///
    /// When a end color is set, the foreground will transition
    /// from the foreground color to this color across the text.
    ///
    /// Requires true color support in the terminal.
    #[inline]
    pub const fn fg_end(mut self, color: Color) -> StyledObject<D> {
        self.style = self.style.fg_end(color);
        self
    }

    /// Sets a background color.
    #[inline]
    pub const fn bg(mut self, color: Color) -> StyledObject<D> {
        self.style = self.style.bg(color);
        self
    }

    /// Sets the background gradient end color.
    ///
    /// When a end color is set, the background will transition
    /// from the background color to this color across the background.
    ///
    /// Requires true color support in the terminal.
    #[inline]
    pub const fn bg_end(mut self, color: Color) -> StyledObject<D> {
        self.style = self.style.bg_end(color);
        self
    }

    /// Adds a attr.
    #[inline]
    pub const fn attr(mut self, attr: Attribute) -> StyledObject<D> {
        self.style = self.style.attr(attr);
        self
    }

    #[inline]
    pub const fn black(self) -> StyledObject<D> {
        self.fg(Color::Black)
    }
    #[inline]
    pub const fn black_end(self) -> StyledObject<D> {
        self.fg_end(Color::Black)
    }
    #[inline]
    pub const fn red(self) -> StyledObject<D> {
        self.fg(Color::Red)
    }
    #[inline]
    pub const fn red_end(self) -> StyledObject<D> {
        self.fg_end(Color::Red)
    }
    #[inline]
    pub const fn green(self) -> StyledObject<D> {
        self.fg(Color::Green)
    }
    #[inline]
    pub const fn green_end(self) -> StyledObject<D> {
        self.fg_end(Color::Green)
    }
    #[inline]
    pub const fn yellow(self) -> StyledObject<D> {
        self.fg(Color::Yellow)
    }
    #[inline]
    pub const fn yellow_end(self) -> StyledObject<D> {
        self.fg_end(Color::Yellow)
    }
    #[inline]
    pub const fn blue(self) -> StyledObject<D> {
        self.fg(Color::Blue)
    }
    #[inline]
    pub const fn blue_end(self) -> StyledObject<D> {
        self.fg_end(Color::Blue)
    }
    #[inline]
    pub const fn magenta(self) -> StyledObject<D> {
        self.fg(Color::Magenta)
    }
    #[inline]
    pub const fn magenta_end(self) -> StyledObject<D> {
        self.fg_end(Color::Magenta)
    }
    #[inline]
    pub const fn cyan(self) -> StyledObject<D> {
        self.fg(Color::Cyan)
    }
    #[inline]
    pub const fn cyan_end(self) -> StyledObject<D> {
        self.fg_end(Color::Cyan)
    }
    #[inline]
    pub const fn white(self) -> StyledObject<D> {
        self.fg(Color::White)
    }
    #[inline]
    pub const fn white_end(self) -> StyledObject<D> {
        self.fg_end(Color::White)
    }
    #[inline]
    pub const fn color256(self, color: u8) -> StyledObject<D> {
        self.fg(Color::Color256(color))
    }
    #[inline]
    pub const fn color256_end(self, color: u8) -> StyledObject<D> {
        self.fg_end(Color::Color256(color))
    }
    #[inline]
    pub const fn true_color(self, r: u8, g: u8, b: u8) -> StyledObject<D> {
        self.fg(Color::TrueColor(r, g, b))
    }
    #[inline]
    pub const fn true_color_end(self, r: u8, g: u8, b: u8) -> StyledObject<D> {
        self.fg_end(Color::TrueColor(r, g, b))
    }

    #[inline]
    pub const fn bright(mut self) -> StyledObject<D> {
        self.style = self.style.bright();
        self
    }

    #[inline]
    pub const fn on_black(self) -> StyledObject<D> {
        self.bg(Color::Black)
    }
    #[inline]
    pub const fn on_black_end(self) -> StyledObject<D> {
        self.bg_end(Color::Black)
    }
    #[inline]
    pub const fn on_red(self) -> StyledObject<D> {
        self.bg(Color::Red)
    }
    #[inline]
    pub const fn on_red_end(self) -> StyledObject<D> {
        self.bg_end(Color::Red)
    }
    #[inline]
    pub const fn on_green(self) -> StyledObject<D> {
        self.bg(Color::Green)
    }
    #[inline]
    pub const fn on_green_end(self) -> StyledObject<D> {
        self.bg_end(Color::Green)
    }
    #[inline]
    pub const fn on_yellow(self) -> StyledObject<D> {
        self.bg(Color::Yellow)
    }
    #[inline]
    pub const fn on_yellow_end(self) -> StyledObject<D> {
        self.bg_end(Color::Yellow)
    }
    #[inline]
    pub const fn on_blue(self) -> StyledObject<D> {
        self.bg(Color::Blue)
    }
    #[inline]
    pub const fn on_blue_end(self) -> StyledObject<D> {
        self.bg_end(Color::Blue)
    }
    #[inline]
    pub const fn on_magenta(self) -> StyledObject<D> {
        self.bg(Color::Magenta)
    }
    #[inline]
    pub const fn on_magenta_end(self) -> StyledObject<D> {
        self.bg_end(Color::Magenta)
    }
    #[inline]
    pub const fn on_cyan(self) -> StyledObject<D> {
        self.bg(Color::Cyan)
    }
    #[inline]
    pub const fn on_cyan_end(self) -> StyledObject<D> {
        self.bg_end(Color::Cyan)
    }
    #[inline]
    pub const fn on_white(self) -> StyledObject<D> {
        self.bg(Color::White)
    }
    #[inline]
    pub const fn on_white_end(self) -> StyledObject<D> {
        self.bg_end(Color::White)
    }
    #[inline]
    pub const fn on_color256(self, color: u8) -> StyledObject<D> {
        self.bg(Color::Color256(color))
    }
    #[inline]
    pub const fn on_color256_end(self, color: u8) -> StyledObject<D> {
        self.bg_end(Color::Color256(color))
    }
    #[inline]
    pub const fn on_true_color(self, r: u8, g: u8, b: u8) -> StyledObject<D> {
        self.bg(Color::TrueColor(r, g, b))
    }
    #[inline]
    pub const fn on_true_color_end(self, r: u8, g: u8, b: u8) -> StyledObject<D> {
        self.bg_end(Color::TrueColor(r, g, b))
    }

    #[inline]
    pub const fn on_bright(mut self) -> StyledObject<D> {
        self.style = self.style.on_bright();
        self
    }

    #[inline]
    pub const fn bold(self) -> StyledObject<D> {
        self.attr(Attribute::Bold)
    }
    #[inline]
    pub const fn dim(self) -> StyledObject<D> {
        self.attr(Attribute::Dim)
    }
    #[inline]
    pub const fn italic(self) -> StyledObject<D> {
        self.attr(Attribute::Italic)
    }
    #[inline]
    pub const fn underlined(self) -> StyledObject<D> {
        self.attr(Attribute::Underlined)
    }
    #[inline]
    pub const fn blink(self) -> StyledObject<D> {
        self.attr(Attribute::Blink)
    }
    #[inline]
    pub const fn blink_fast(self) -> StyledObject<D> {
        self.attr(Attribute::BlinkFast)
    }
    #[inline]
    pub const fn reverse(self) -> StyledObject<D> {
        self.attr(Attribute::Reverse)
    }
    #[inline]
    pub const fn hidden(self) -> StyledObject<D> {
        self.attr(Attribute::Hidden)
    }
    #[inline]
    pub const fn strikethrough(self) -> StyledObject<D> {
        self.attr(Attribute::StrikeThrough)
    }

    #[inline]
    pub const fn size_hint(mut self, size_hint: Option<usize>) -> Self {
        self.size_hint = size_hint;
        self
    }
    #[inline]
    pub const fn position_hint(mut self, position_hint: Option<usize>) -> Self {
        self.position_hint = position_hint;
        self
    }
}

macro_rules! impl_fmt {
    ($name:ident) => {
        impl<D: fmt::$name> fmt::$name for StyledObject<D> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let use_colors = self
                    .style
                    .force
                    .unwrap_or_else(|| match self.style.for_stderr {
                        true => colors_enabled_stderr(),
                        false => colors_enabled(),
                    });

                let fg_gradient = self.style.get_fg_gradient();
                let bg_gradient = self.style.get_bg_gradient();

                let use_gradient = use_colors && (fg_gradient.is_some() || bg_gradient.is_some());

                if use_gradient {
                    struct DisplayWrapper<'a, D>(&'a D);

                    impl<'a, D> fmt::Display for DisplayWrapper<'a, D>
                    where
                        D: fmt::$name,
                    {
                        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                            self.0.fmt(f)
                        }
                    }

                    let text = DisplayWrapper(&self.val).to_string();

                    fmt_gradient(
                        f,
                        &text,
                        fg_gradient,
                        bg_gradient,
                        self.size_hint,
                        self.position_hint,
                        self.style.attrs,
                    )?;

                    return Ok(());
                }

                let mut reset = false;
                if use_colors {
                    if let Some(fg) = self.style.fg {
                        if let Color::TrueColor(r, g, b) = fg {
                            write!(f, "\x1b[38;2;{};{};{}m", r, g, b)?;
                        } else if fg.is_color256() {
                            write!(f, "\x1b[38;5;{}m", fg.ansi_num())?;
                        } else if self.style.fg_bright {
                            write!(f, "\x1b[38;5;{}m", fg.ansi_num() + 8)?;
                        } else {
                            write!(f, "\x1b[{}m", fg.ansi_num() + 30)?;
                        }
                        reset = true;
                    }
                    if let Some(bg) = self.style.bg {
                        if let Color::TrueColor(r, g, b) = bg {
                            write!(f, "\x1b[48;2;{};{};{}m", r, g, b)?;
                        } else if bg.is_color256() {
                            write!(f, "\x1b[48;5;{}m", bg.ansi_num())?;
                        } else if self.style.bg_bright {
                            write!(f, "\x1b[48;5;{}m", bg.ansi_num() + 8)?;
                        } else {
                            write!(f, "\x1b[{}m", bg.ansi_num() + 40)?;
                        }
                        reset = true;
                    }
                    if !self.style.attrs.is_empty() {
                        write!(f, "{}", self.style.attrs)?;
                        reset = true;
                    }
                }
                fmt::$name::fmt(&self.val, f)?;
                if reset {
                    write!(f, "\x1b[0m")?;
                }
                Ok(())
            }
        }
    };
}

impl_fmt!(Binary);
impl_fmt!(Debug);
impl_fmt!(Display);
impl_fmt!(LowerExp);
impl_fmt!(LowerHex);
impl_fmt!(Octal);
impl_fmt!(Pointer);
impl_fmt!(UpperExp);
impl_fmt!(UpperHex);

struct Gradient {
    start: Color,
    end: Color,
}

struct FgGradient(Gradient);
struct BgGradient(Gradient);

/// Formats text with gradient colors.
fn fmt_gradient(
    f: &mut fmt::Formatter,
    text: &str,
    fg_gradient: Option<FgGradient>,
    bg_gradient: Option<BgGradient>,
    size_hint: Option<usize>,
    position_hint: Option<usize>,
    attrs: Attributes,
) -> fmt::Result {
    let char_count = size_hint.unwrap_or(measure_text_width(text));

    if char_count == 0 {
        return Ok(());
    }

    let mut visible_idx = position_hint.unwrap_or_default();

    #[cfg(feature = "ansi-parsing")]
    fn escape_iterator(text: &str) -> impl Iterator<Item = (&str, bool)> {
        AnsiCodeIterator::new(text)
    }

    // Assume there are no existing escape codes when ansi-parsing feature is disabled.
    #[cfg(not(feature = "ansi-parsing"))]
    fn escape_iterator(text: &str) -> impl Iterator<Item = (&str, bool)> {
        [text].into_iter().map(|s| (s, false))
    }

    for (chars, is_escape) in escape_iterator(text) {
        if is_escape {
            write!(f, "{chars}")?;

            continue;
        }

        for ch in chars.chars() {
            // Calculate interpolation factor
            let t = if char_count == 1 {
                0.0
            } else {
                visible_idx as f32 / (char_count - 1) as f32
            };

            // Apply foreground gradient
            if let Some(FgGradient(Gradient { start, end })) = fg_gradient {
                let (r, g, b) = start.interpolate(end, t);

                write!(f, "\x1b[38;2;{r};{g};{b}m")?;
            };

            // Apply background gradient
            if let Some(BgGradient(Gradient { start, end })) = bg_gradient {
                let (r, g, b) = start.interpolate(end, t);

                write!(f, "\x1b[48;2;{r};{g};{b}m")?;
            }

            // Apply attributes (only needed once per char since we're resetting)
            if !attrs.is_empty() {
                write!(f, "{attrs}")?;
            }

            write!(f, "{ch}")?;

            visible_idx += 1;
        }
    }

    // Reset at the end
    write!(f, "\x1b[0m")?;

    Ok(())
}

/// "Intelligent" emoji formatter.
///
/// This struct intelligently wraps an emoji so that it is rendered
/// only on systems that want emojis and renders a fallback on others.
///
/// Example:
///
/// ```rust
/// use console::Emoji;
/// println!("[3/4] {}Downloading ...", Emoji("🚚 ", ""));
/// println!("[4/4] {} Done!", Emoji("✨", ":-)"));
/// ```
#[derive(Copy, Clone)]
pub struct Emoji<'a, 'b>(pub &'a str, pub &'b str);

impl<'a, 'b> Emoji<'a, 'b> {
    pub fn new(emoji: &'a str, fallback: &'b str) -> Emoji<'a, 'b> {
        Emoji(emoji, fallback)
    }
}

impl fmt::Display for Emoji<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if wants_emoji() {
            write!(f, "{}", self.0)
        } else {
            write!(f, "{}", self.1)
        }
    }
}

fn str_width(s: &str) -> usize {
    #[cfg(feature = "unicode-width")]
    {
        use unicode_width::UnicodeWidthStr;
        s.width()
    }
    #[cfg(not(feature = "unicode-width"))]
    {
        s.chars().count()
    }
}

#[cfg(feature = "ansi-parsing")]
pub(crate) fn char_width(c: char) -> usize {
    #[cfg(feature = "unicode-width")]
    {
        use unicode_width::UnicodeWidthChar;
        c.width().unwrap_or(0)
    }
    #[cfg(not(feature = "unicode-width"))]
    {
        let _c = c;
        1
    }
}

#[cfg(not(feature = "ansi-parsing"))]
pub(crate) fn char_width(_c: char) -> usize {
    1
}

/// Truncates a string to a certain number of characters.
///
/// This ensures that escape codes are not screwed up in the process.
/// If the maximum length is hit the string will be truncated but
/// escapes code will still be honored.  If truncation takes place
/// the tail string will be appended.
pub fn truncate_str<'a>(s: &'a str, width: usize, tail: &str) -> Cow<'a, str> {
    if measure_text_width(s) <= width {
        return Cow::Borrowed(s);
    }

    #[cfg(feature = "ansi-parsing")]
    {
        use core::cmp::Ordering;
        let mut iter = AnsiCodeIterator::new(s);
        let mut length = 0;
        let mut rv = None;

        while let Some(item) = iter.next() {
            match item {
                (s, false) => {
                    if rv.is_none() {
                        if str_width(s) + length > width.saturating_sub(str_width(tail)) {
                            let ts = iter.current_slice();

                            let mut s_byte = 0;
                            let mut s_width = 0;
                            let rest_width =
                                width.saturating_sub(str_width(tail)).saturating_sub(length);
                            for c in s.chars() {
                                s_byte += c.len_utf8();
                                s_width += char_width(c);
                                match s_width.cmp(&rest_width) {
                                    Ordering::Equal => break,
                                    Ordering::Greater => {
                                        s_byte -= c.len_utf8();
                                        break;
                                    }
                                    Ordering::Less => continue,
                                }
                            }

                            let idx = ts.len() - s.len() + s_byte;
                            let mut buf = ts[..idx].to_string();
                            buf.push_str(tail);
                            rv = Some(buf);
                        }
                        length += str_width(s);
                    }
                }
                (s, true) => {
                    if let Some(ref mut rv) = rv {
                        rv.push_str(s);
                    }
                }
            }
        }

        if let Some(buf) = rv {
            Cow::Owned(buf)
        } else {
            Cow::Borrowed(s)
        }
    }

    #[cfg(not(feature = "ansi-parsing"))]
    {
        Cow::Owned(format!(
            "{}{}",
            &s[..width.saturating_sub(tail.len())],
            tail
        ))
    }
}

/// Pads a string to fill a certain number of characters.
///
/// This will honor ansi codes correctly and allows you to align a string
/// on the left, right or centered.  Additionally truncation can be enabled
/// by setting `truncate` to a string that should be used as a truncation
/// marker.
pub fn pad_str<'a>(
    s: &'a str,
    width: usize,
    align: Alignment,
    truncate: Option<&str>,
) -> Cow<'a, str> {
    pad_str_with(s, width, align, truncate, ' ')
}
/// Pads a string with specific padding to fill a certain number of characters.
///
/// This will honor ansi codes correctly and allows you to align a string
/// on the left, right or centered.  Additionally truncation can be enabled
/// by setting `truncate` to a string that should be used as a truncation
/// marker.
pub fn pad_str_with<'a>(
    s: &'a str,
    width: usize,
    align: Alignment,
    truncate: Option<&str>,
    pad: char,
) -> Cow<'a, str> {
    let cols = measure_text_width(s);

    if cols >= width {
        return match truncate {
            None => Cow::Borrowed(s),
            Some(tail) => truncate_str(s, width, tail),
        };
    }

    let diff = width - cols;

    let (left_pad, right_pad) = match align {
        Alignment::Left => (0, diff),
        Alignment::Right => (diff, 0),
        Alignment::Center => (diff / 2, diff - diff / 2),
    };

    let mut rv = String::new();
    for _ in 0..left_pad {
        rv.push(pad);
    }
    rv.push_str(s);
    for _ in 0..right_pad {
        rv.push(pad);
    }
    Cow::Owned(rv)
}

#[test]
fn test_text_width() {
    let s = style("foo")
        .red()
        .on_black()
        .bold()
        .force_styling(true)
        .to_string();

    assert_eq!(
        measure_text_width(&s),
        if cfg!(feature = "ansi-parsing") {
            3
        } else {
            21
        }
    );

    let s = style("🐶 <3").red().force_styling(true).to_string();

    assert_eq!(
        measure_text_width(&s),
        match (
            cfg!(feature = "ansi-parsing"),
            cfg!(feature = "unicode-width")
        ) {
            (true, true) => 5,    // "🐶 <3"
            (true, false) => 4,   // "🐶 <3", no unicode-aware width
            (false, true) => 14,  // full string
            (false, false) => 13, // full string, no unicode-aware width
        }
    );
}

#[test]
#[cfg(all(feature = "unicode-width", feature = "ansi-parsing"))]
fn test_truncate_str() {
    let s = format!("foo {}", style("bar").red().force_styling(true));
    assert_eq!(
        &truncate_str(&s, 5, ""),
        &format!("foo {}", style("b").red().force_styling(true))
    );
    let s = format!("foo {}", style("bar").red().force_styling(true));
    assert_eq!(
        &truncate_str(&s, 5, "!"),
        &format!("foo {}", style("!").red().force_styling(true))
    );
    let s = format!("foo {} baz", style("bar").red().force_styling(true));
    assert_eq!(
        &truncate_str(&s, 10, "..."),
        &format!("foo {}...", style("bar").red().force_styling(true))
    );
    let s = format!("foo {}", style("バー").red().force_styling(true));
    assert_eq!(
        &truncate_str(&s, 5, ""),
        &format!("foo {}", style("").red().force_styling(true))
    );
    let s = format!("foo {}", style("バー").red().force_styling(true));
    assert_eq!(
        &truncate_str(&s, 6, ""),
        &format!("foo {}", style("バ").red().force_styling(true))
    );
    let s = format!("foo {}", style("バー").red().force_styling(true));
    assert_eq!(
        &truncate_str(&s, 2, "!!!"),
        &format!("!!!{}", style("").red().force_styling(true))
    );
}

#[test]
fn test_truncate_str_no_ansi() {
    assert_eq!(&truncate_str("foo bar", 7, "!"), "foo bar");
    assert_eq!(&truncate_str("foo bar", 5, ""), "foo b");
    assert_eq!(&truncate_str("foo bar", 5, "!"), "foo !");
    assert_eq!(&truncate_str("foo bar baz", 10, "..."), "foo bar...");
    assert_eq!(&truncate_str("foo bar", 0, ""), "");
    assert_eq!(&truncate_str("foo bar", 0, "!"), "!");
    assert_eq!(&truncate_str("foo bar", 2, "!!!"), "!!!");
    assert_eq!(&truncate_str("ab", 2, "!!!"), "ab");
}

#[test]
fn test_pad_str() {
    assert_eq!(pad_str("foo", 7, Alignment::Center, None), "  foo  ");
    assert_eq!(pad_str("foo", 7, Alignment::Left, None), "foo    ");
    assert_eq!(pad_str("foo", 7, Alignment::Right, None), "    foo");
    assert_eq!(pad_str("foo", 3, Alignment::Left, None), "foo");
    assert_eq!(pad_str("foobar", 3, Alignment::Left, None), "foobar");
    assert_eq!(pad_str("foobar", 3, Alignment::Left, Some("")), "foo");
    assert_eq!(
        pad_str("foobarbaz", 6, Alignment::Left, Some("...")),
        "foo..."
    );
}

#[test]
fn test_pad_str_with() {
    assert_eq!(
        pad_str_with("foo", 7, Alignment::Center, None, '#'),
        "##foo##"
    );
    assert_eq!(
        pad_str_with("foo", 7, Alignment::Left, None, '#'),
        "foo####"
    );
    assert_eq!(
        pad_str_with("foo", 7, Alignment::Right, None, '#'),
        "####foo"
    );
    assert_eq!(pad_str_with("foo", 3, Alignment::Left, None, '#'), "foo");
    assert_eq!(
        pad_str_with("foobar", 3, Alignment::Left, None, '#'),
        "foobar"
    );
    assert_eq!(
        pad_str_with("foobar", 3, Alignment::Left, Some(""), '#'),
        "foo"
    );
    assert_eq!(
        pad_str_with("foobarbaz", 6, Alignment::Left, Some("..."), '#'),
        "foo..."
    );
}

#[test]
fn test_attributes_single() {
    for attr in Attribute::MAP {
        let attrs = Attributes::new().insert(attr);
        assert_eq!(attrs.bits().collect::<Vec<_>>(), [attr as u16]);
        assert_eq!(attrs.attrs().collect::<Vec<_>>(), [attr]);
        assert_eq!(format!("{attrs:?}"), format!("{{{:?}}}", attr));
    }
}

#[test]
fn test_attributes_many() {
    let tests: [&[Attribute]; 3] = [
        &[
            Attribute::Bold,
            Attribute::Underlined,
            Attribute::BlinkFast,
            Attribute::Hidden,
        ],
        &[
            Attribute::Dim,
            Attribute::Italic,
            Attribute::Blink,
            Attribute::Reverse,
            Attribute::StrikeThrough,
        ],
        &Attribute::MAP,
    ];
    for test_attrs in tests {
        let mut attrs = Attributes::new();
        for attr in test_attrs {
            attrs = attrs.insert(*attr);
        }
        assert_eq!(
            attrs.bits().collect::<Vec<_>>(),
            test_attrs
                .iter()
                .map(|attr| *attr as u16)
                .collect::<Vec<_>>()
        );
        assert_eq!(&attrs.attrs().collect::<Vec<_>>(), test_attrs);
    }
}

#[test]
fn test_color_to_rgb() {
    assert_eq!(Color::Black.to_rgb(), (0, 0, 0));
    assert_eq!(Color::Red.to_rgb(), (205, 49, 49));
    assert_eq!(Color::White.to_rgb(), (229, 229, 229));
    assert_eq!(Color::TrueColor(100, 150, 200).to_rgb(), (100, 150, 200));
    assert_eq!(Color::Color256(0).to_rgb(), (0, 0, 0));
    assert_eq!(Color::Color256(15).to_rgb(), (255, 255, 255));
}

#[test]
fn test_color_interpolate() {
    let start = Color::TrueColor(0, 0, 0);
    let end = Color::TrueColor(255, 255, 255);

    // At t=0, should be start color
    assert_eq!(start.interpolate(end, 0.0), (0, 0, 0));

    // At t=1, should be end color
    assert_eq!(start.interpolate(end, 1.0), (255, 255, 255));

    // At t=0.5, should be midpoint
    assert_eq!(start.interpolate(end, 0.5), (128, 128, 128));
}

#[test]
fn test_style_gradient_builder() {
    let style = Style::new().fg(Color::Red).fg_end(Color::Blue);

    assert!(style.get_fg_gradient().is_some());
    assert!(style.get_bg_gradient().is_none());
}

#[test]
fn test_style_from_dotted_str_gradient() {
    let style = Style::from_dotted_str("#ff0000->#00ff00");

    assert!(style.get_fg_gradient().is_some());

    let style = Style::from_dotted_str("on_#ff0000->#0000ff");
    assert!(style.get_bg_gradient().is_some());

    let style = Style::from_dotted_str("#ff0000->#00ff00.on_#0000ff->#ff00ff");
    assert!(style.get_fg_gradient().is_some());
    assert!(style.get_bg_gradient().is_some());
}

#[test]
fn test_gradient_styled_output() {
    let styled = style("AB")
        .fg(Color::TrueColor(255, 0, 0))
        .fg_end(Color::TrueColor(0, 0, 255))
        .force_styling(true);

    assert_eq!(
        styled.to_string(),
        "\u{1b}[38;2;255;0;0mA\u{1b}[38;2;0;0;255mB\u{1b}[0m"
    );
}

#[test]
fn test_gradient_single_char() {
    // Single character should just use start color
    let styled = style("X")
        .fg(Color::TrueColor(255, 0, 0))
        .fg_end(Color::TrueColor(0, 0, 255))
        .force_styling(true);

    let output = format!("{}", styled);
    // Should contain the start color (255, 0, 0)
    assert!(output.contains("\x1b[38;2;255;0;0m"));
}

#[test]
fn test_gradient_empty_string() {
    let styled = style("")
        .fg(Color::TrueColor(255, 0, 0))
        .fg_end(Color::TrueColor(0, 0, 255))
        .force_styling(true);

    assert_eq!(styled.to_string(), "");
}
