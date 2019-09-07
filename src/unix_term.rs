use std::fmt::Display;
use std::fs;
use std::io;
use std::io::{BufRead, BufReader};
use std::mem;
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

pub fn terminal_size() -> Option<(u16, u16)> {
    unsafe {
        if libc::isatty(libc::STDOUT_FILENO) != 1 {
            return None;
        }

        let mut winsize: libc::winsize = mem::zeroed();

        // FIXME: ".into()" used as a temporary fix for a libc bug
        // https://github.com/rust-lang/libc/pull/704
        #[allow(clippy::identity_conversion)]
        libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ.into(), &mut winsize);
        if winsize.ws_row > 0 && winsize.ws_col > 0 {
            Some((winsize.ws_row as u16, winsize.ws_col as u16))
        } else {
            None
        }
    }
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
        let read = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, 20);
        if read < 0 {
            Err(io::Error::last_os_error())
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
