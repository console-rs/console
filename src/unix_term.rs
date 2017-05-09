use std::io;
use std::fs;
use std::mem;
use std::os::unix::io::AsRawFd;

use libc;
use termios;

use term::Term;

pub const DEFAULT_WIDTH: u16 = 80;


#[inline(always)]
pub fn is_a_terminal(out: &Term) -> bool {
    unsafe {
        libc::isatty(out.as_raw_fd()) == 1
    }
}

pub fn terminal_size() -> Option<(u16, u16)> {
    unsafe {
        if libc::isatty(libc::STDOUT_FILENO) != 1 {
            return None;
        }

        let mut winsize: libc::winsize = mem::zeroed();
        libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize);
        if winsize.ws_row > 0 && winsize.ws_col > 0 {
            Some((winsize.ws_row as u16, winsize.ws_col as u16))
        } else {
            None
        }
    }
}

pub fn move_cursor_down(out: &Term, n: usize) -> io::Result<()> {
    if n > 0 {
        out.write_str(&format!("\x1b[{}B", n))
    } else {
        Ok(())
    }
}

pub fn move_cursor_up(out: &Term, n: usize) -> io::Result<()> {
    if n > 0 {
        out.write_str(&format!("\x1b[{}A", n))
    } else {
        Ok(())
    }
}

pub fn clear_line(out: &Term) -> io::Result<()> {
    out.write_str(&format!("\r\x1b[2K"))
}

pub fn read_single_char() -> io::Result<char> {
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
    let original = termios.clone();
    termios::cfmakeraw(&mut termios);
    termios::tcsetattr(fd, termios::TCSADRAIN, &termios)?;
    let rv = unsafe {
        let read = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, 20);
        if read < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(buf[0] as char)
        }
    };
    termios::tcsetattr(fd, termios::TCSADRAIN, &original)?;

    // if the user hit ^C we want to signal SIGINT to outselves.
    if let Ok('\x03') = rv {
        unsafe { libc::raise(libc::SIGINT); }
    }

    rv
}

pub fn wants_emoji() -> bool {
    cfg!(target_os = "macos")
}
