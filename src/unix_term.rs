use std::env;
use std::fmt::Display;
use std::fs;
use std::io::{self, BufRead, BufReader, Read};
use std::mem;
use std::os::fd::{AsRawFd, RawFd};
use std::str;

#[cfg(not(target_os = "macos"))]
use once_cell::sync::Lazy;

use crate::kb::Key;
use crate::term::Term;

pub(crate) use crate::common_term::*;

pub(crate) const DEFAULT_WIDTH: u16 = 80;

#[inline]
pub(crate) fn is_a_terminal(out: &impl AsRawFd) -> bool {
    unsafe { libc::isatty(out.as_raw_fd()) != 0 }
}

pub(crate) fn is_a_color_terminal(out: &Term) -> bool {
    if !is_a_terminal(out) {
        return false;
    }

    if env::var("NO_COLOR").is_ok() {
        return false;
    }

    match env::var("TERM") {
        Ok(term) => term != "dumb",
        Err(_) => false,
    }
}

fn c_result<F: FnOnce() -> libc::c_int>(f: F) -> io::Result<()> {
    let res = f();
    if res != 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub(crate) fn terminal_size(out: &Term) -> Option<(u16, u16)> {
    if !is_a_terminal(out) {
        return None;
    }
    let winsize = unsafe {
        let mut winsize: libc::winsize = mem::zeroed();

        // FIXME: ".into()" used as a temporary fix for a libc bug
        // https://github.com/rust-lang/libc/pull/704
        #[allow(clippy::useless_conversion)]
        libc::ioctl(out.as_raw_fd(), libc::TIOCGWINSZ.into(), &mut winsize);
        winsize
    };
    if winsize.ws_row > 0 && winsize.ws_col > 0 {
        Some((winsize.ws_row, winsize.ws_col))
    } else {
        None
    }
}

enum Input<T> {
    Stdin(io::Stdin),
    File(T),
}

impl Input<BufReader<fs::File>> {
    fn buffered() -> io::Result<Self> {
        Ok(match Input::unbuffered()? {
            Input::Stdin(s) => Input::Stdin(s),
            Input::File(f) => Input::File(BufReader::new(f)),
        })
    }
}

impl Input<fs::File> {
    fn unbuffered() -> io::Result<Self> {
        let stdin = io::stdin();
        if is_a_terminal(&stdin) {
            Ok(Input::Stdin(stdin))
        } else {
            let f = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")?;
            Ok(Input::File(f))
        }
    }
}

impl<T: Read> Read for Input<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Stdin(s) => s.read(buf),
            Self::File(f) => f.read(buf),
        }
    }
}

// NB: this is not a full BufRead implementation because io::Stdin does not implement BufRead.
impl<T: BufRead> Input<T> {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        match self {
            Self::Stdin(s) => s.read_line(buf),
            Self::File(f) => f.read_line(buf),
        }
    }
}

impl AsRawFd for Input<fs::File> {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Self::Stdin(s) => s.as_raw_fd(),
            Self::File(f) => f.as_raw_fd(),
        }
    }
}

impl AsRawFd for Input<BufReader<fs::File>> {
    fn as_raw_fd(&self) -> RawFd {
        match self {
            Self::Stdin(s) => s.as_raw_fd(),
            Self::File(f) => f.get_ref().as_raw_fd(),
        }
    }
}

pub(crate) fn read_secure() -> io::Result<String> {
    let mut input = Input::buffered()?;

    let mut termios = mem::MaybeUninit::uninit();
    c_result(|| unsafe { libc::tcgetattr(input.as_raw_fd(), termios.as_mut_ptr()) })?;
    let mut termios = unsafe { termios.assume_init() };
    let original = termios;
    termios.c_lflag &= !libc::ECHO;
    c_result(|| unsafe { libc::tcsetattr(input.as_raw_fd(), libc::TCSAFLUSH, &termios) })?;
    let mut rv = String::new();

    let read_rv = input.read_line(&mut rv);

    c_result(|| unsafe { libc::tcsetattr(input.as_raw_fd(), libc::TCSAFLUSH, &original) })?;

    read_rv.map(|_| {
        let len = rv.trim_end_matches(&['\r', '\n'][..]).len();
        rv.truncate(len);
        rv
    })
}

fn read_single_char<T: Read + AsRawFd>(input: &mut T) -> io::Result<Option<char>> {
    let original = unsafe { libc::fcntl(input.as_raw_fd(), libc::F_GETFL) };
    c_result(|| unsafe {
        libc::fcntl(
            input.as_raw_fd(),
            libc::F_SETFL,
            original | libc::O_NONBLOCK,
        )
    })?;
    let mut buf = [0u8; 1];
    let result = read_bytes(input, &mut buf);
    c_result(|| unsafe { libc::fcntl(input.as_raw_fd(), libc::F_SETFL, original) })?;
    match result {
        Ok(()) => {
            let [byte] = buf;
            Ok(Some(byte as char))
        }
        Err(err) => {
            if err.kind() == io::ErrorKind::WouldBlock {
                Ok(None)
            } else {
                Err(err)
            }
        }
    }
}

fn read_bytes(input: &mut impl Read, buf: &mut [u8]) -> io::Result<()> {
    input.read_exact(buf)?;
    if buf.starts_with(b"\x03") {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "read interrupted",
        ))
    } else {
        Ok(())
    }
}

fn read_single_key_impl<T: Read + AsRawFd>(fd: &mut T) -> Result<Key, io::Error> {
    // NB: this doesn't use `read_single_char` because we want a blocking read here.
    let mut buf = [0u8; 1];
    read_bytes(fd, &mut buf)?;
    let [byte] = buf;
    match byte {
        b'\x1b' => {
            // Escape was read, keep reading in case we find a familiar key
            if let Some(c1) = read_single_char(fd)? {
                if c1 == '[' {
                    if let Some(c2) = read_single_char(fd)? {
                        match c2 {
                            'A' => Ok(Key::ArrowUp),
                            'B' => Ok(Key::ArrowDown),
                            'C' => Ok(Key::ArrowRight),
                            'D' => Ok(Key::ArrowLeft),
                            'H' => Ok(Key::Home),
                            'F' => Ok(Key::End),
                            'Z' => Ok(Key::BackTab),
                            _ => {
                                let c3 = read_single_char(fd)?;
                                if let Some(c3) = c3 {
                                    if c3 == '~' {
                                        match c2 {
                                            '1' => Ok(Key::Home), // tmux
                                            '2' => Ok(Key::Insert),
                                            '3' => Ok(Key::Del),
                                            '4' => Ok(Key::End), // tmux
                                            '5' => Ok(Key::PageUp),
                                            '6' => Ok(Key::PageDown),
                                            '7' => Ok(Key::Home), // xrvt
                                            '8' => Ok(Key::End),  // xrvt
                                            _ => Ok(Key::UnknownEscSeq(vec![c1, c2, c3])),
                                        }
                                    } else {
                                        Ok(Key::UnknownEscSeq(vec![c1, c2, c3]))
                                    }
                                } else {
                                    // \x1b[ and 1 more char
                                    Ok(Key::UnknownEscSeq(vec![c1, c2]))
                                }
                            }
                        }
                    } else {
                        // \x1b[ and no more input
                        Ok(Key::UnknownEscSeq(vec![c1]))
                    }
                } else {
                    // char after escape is not [
                    Ok(Key::UnknownEscSeq(vec![c1]))
                }
            } else {
                //nothing after escape
                Ok(Key::Escape)
            }
        }
        byte => {
            let mut buf: [u8; 4] = [byte, 0, 0, 0];
            if byte & 224u8 == 192u8 {
                // a two byte unicode character
                read_bytes(fd, &mut buf[1..][..1])?;
                Ok(key_from_utf8(&buf[..2]))
            } else if byte & 240u8 == 224u8 {
                // a three byte unicode character
                read_bytes(fd, &mut buf[1..][..2])?;
                Ok(key_from_utf8(&buf[..3]))
            } else if byte & 248u8 == 240u8 {
                // a four byte unicode character
                read_bytes(fd, &mut buf[1..][..3])?;
                Ok(key_from_utf8(&buf[..4]))
            } else {
                Ok(match byte as char {
                    '\n' | '\r' => Key::Enter,
                    '\x7f' => Key::Backspace,
                    '\t' => Key::Tab,
                    '\x01' => Key::Home,      // Control-A (home)
                    '\x05' => Key::End,       // Control-E (end)
                    '\x08' => Key::Backspace, // Control-H (8) (Identical to '\b')
                    c => Key::Char(c),
                })
            }
        }
    }
}

pub(crate) fn read_single_key(ctrlc_key: bool) -> io::Result<Key> {
    let mut input = Input::unbuffered()?;

    let mut termios = core::mem::MaybeUninit::uninit();
    c_result(|| unsafe { libc::tcgetattr(input.as_raw_fd(), termios.as_mut_ptr()) })?;
    let mut termios = unsafe { termios.assume_init() };
    let original = termios;
    unsafe { libc::cfmakeraw(&mut termios) };
    termios.c_oflag = original.c_oflag;
    c_result(|| unsafe { libc::tcsetattr(input.as_raw_fd(), libc::TCSADRAIN, &termios) })?;
    let rv = read_single_key_impl(&mut input);
    c_result(|| unsafe { libc::tcsetattr(input.as_raw_fd(), libc::TCSADRAIN, &original) })?;

    // if the user hit ^C we want to signal SIGINT to ourselves.
    if let Err(ref err) = rv {
        if err.kind() == io::ErrorKind::Interrupted {
            if !ctrlc_key {
                unsafe {
                    libc::raise(libc::SIGINT);
                }
            } else {
                return Ok(Key::CtrlC);
            }
        }
    }

    rv
}

fn key_from_utf8(buf: &[u8]) -> Key {
    if let Ok(s) = str::from_utf8(buf) {
        if let Some(c) = s.chars().next() {
            return Key::Char(c);
        }
    }
    Key::Unknown
}

#[cfg(not(target_os = "macos"))]
static IS_LANG_UTF8: Lazy<bool> = Lazy::new(|| match std::env::var("LANG") {
    Ok(lang) => lang.to_uppercase().ends_with("UTF-8"),
    _ => false,
});

#[cfg(target_os = "macos")]
pub(crate) fn wants_emoji() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn wants_emoji() -> bool {
    *IS_LANG_UTF8
}

pub(crate) fn set_title<T: Display>(title: T) {
    print!("\x1b]0;{}\x07", title);
}
