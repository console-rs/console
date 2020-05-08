use std::env;
use std::fmt::Display;
use std::fs;
use std::io;
use std::io::{BufRead, BufReader};
use std::os::unix::io::AsRawFd;
use std::str;

use crate::kb::Key;
use crate::term::Term;

pub use crate::common_term::*;

pub const DEFAULT_WIDTH: u16 = 80;

#[inline]
pub fn is_a_terminal(out: &Term) -> bool {
    unsafe { libc::isatty(out.as_raw_fd()) != 0 }
}

pub fn is_a_color_terminal(out: &Term) -> bool {
    if !is_a_terminal(out) {
        return false;
    }

    match env::var("TERM") {
        Ok(term) => term != "dumb",
        Err(_) => false,
    }
}

#[inline]
pub fn terminal_size() -> Option<(u16, u16)> {
    terminal_size::terminal_size().map(|x| ((x.1).0, (x.0).0))
}

pub fn read_secure() -> io::Result<String> {
    let f_tty;
    let fd = unsafe {
        if libc::isatty(libc::STDIN_FILENO) == 1 {
            f_tty = None;
            libc::STDIN_FILENO
        } else {
            let f = fs::File::open("/dev/tty")?;
            let fd = f.as_raw_fd();
            f_tty = Some(BufReader::new(f));
            fd
        }
    };

    let mut termios = termios::Termios::from_fd(fd)?;
    let original = termios;
    termios.c_lflag &= !termios::ECHO;
    termios::tcsetattr(fd, termios::TCSAFLUSH, &termios)?;
    let mut rv = String::new();

    let read_rv = if let Some(mut f) = f_tty {
        f.read_line(&mut rv)
    } else {
        io::stdin().read_line(&mut rv)
    };

    termios::tcsetattr(fd, termios::TCSAFLUSH, &original)?;

    read_rv.map(|_| {
        let len = rv.trim_end_matches(&['\r', '\n'][..]).len();
        rv.truncate(len);
        rv
    })
}

pub fn read_single_key() -> io::Result<Key> {
    let tty_f;
    let fd = unsafe {
        if libc::isatty(libc::STDIN_FILENO) == 1 {
            libc::STDIN_FILENO
        } else {
            tty_f = fs::File::open("/dev/tty")?;
            tty_f.as_raw_fd()
        }
    };
    let mut buf = [0u8; 20];
    let mut termios = termios::Termios::from_fd(fd)?;
    let original = termios;
    termios::cfmakeraw(&mut termios);
    termios::tcsetattr(fd, termios::TCSADRAIN, &termios)?;
    let rv = unsafe {
        let read = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, 1);
        if read < 0 {
            Err(io::Error::last_os_error())
        } else if buf[0] == b'\x1b' {
            // read 19 more bytes if the first byte was the ESC code
            let read = libc::read(fd, buf[1..].as_mut_ptr() as *mut libc::c_void, 19);
            if read < 0 {
                Err(io::Error::last_os_error())
            } else if buf[1] == b'\x03' {
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "read interrupted",
                ))
            } else {
                Ok(key_from_escape_codes(&buf[..(read+1) as usize]))
            }
        } else if buf[0] & 224u8 == 192u8 { 
            // a two byte unicode character
            let read = libc::read(fd, buf[1..].as_mut_ptr() as *mut libc::c_void, 1);
            if read < 0 {
                Err(io::Error::last_os_error())
            } else if buf[1] == b'\x03' {
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "read interrupted",
                ))
            } else {
                Ok(key_from_escape_codes(&buf[..2 as usize]))
            }
        } else if buf[0] & 240u8 == 224u8 { 
            // a three byte unicode character
            let read = libc::read(fd, buf[1..].as_mut_ptr() as *mut libc::c_void, 2);
            if read < 0 {
                Err(io::Error::last_os_error())
            } else if buf[1] == b'\x03' {
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "read interrupted",
                ))
            } else {
                Ok(key_from_escape_codes(&buf[..3 as usize]))
            }
        } else if buf[0] & 248u8 == 240u8 { 
            // a four byte unicode character
            let read = libc::read(fd, buf[1..].as_mut_ptr() as *mut libc::c_void, 3);
            if read < 0 {
                Err(io::Error::last_os_error())
            } else if buf[1] == b'\x03' {
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "read interrupted",
                ))
            } else {
                Ok(key_from_escape_codes(&buf[..4 as usize]))
            }
        } else if buf[0] == b'\x03' {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "read interrupted",
            ))
        } else {
            Ok(key_from_escape_codes(&buf[..read as usize]))
        }
    };
    termios::tcsetattr(fd, termios::TCSADRAIN, &original)?;

    // if the user hit ^C we want to signal SIGINT to outselves.
    if let Err(ref err) = rv {
        if err.kind() == io::ErrorKind::Interrupted {
            unsafe {
                libc::raise(libc::SIGINT);
            }
        }
    }

    rv
}

pub fn key_from_escape_codes(buf: &[u8]) -> Key {
    match buf {
        b"\x1b[D" => Key::ArrowLeft,
        b"\x1b[C" => Key::ArrowRight,
        b"\x1b[A" => Key::ArrowUp,
        b"\x1b[B" => Key::ArrowDown,
        b"\n" | b"\r" => Key::Enter,
        b"\x1b" => Key::Escape,
        b"\x7f" => Key::Backspace,
        b"\x1b[H" => Key::Home,
        b"\x1b[F" => Key::End,
        b"\t" => Key::Tab,
        b"\x1b[3~" => Key::Del,
        buf => {
            if let Ok(s) = str::from_utf8(buf) {
                if let Some(c) = s.chars().next() {
                    return Key::Char(c);
                }
            }
            Key::Unknown
        }
    }
}

pub fn wants_emoji() -> bool {
    cfg!(target_os = "macos")
}

pub fn set_title<T: Display>(title: T) {
    print!("\x1b]0;{}\x07", title);
}
