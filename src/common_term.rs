use std::io;

use crate::term::Term;

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
pub fn move_cursor_left(out: &Term, n: usize) -> io::Result<()> {
    if n > 0 {
        out.write_str(&format!("\x1b[{}D", n))
    } else {
        Ok(())
    }
}

pub fn move_cursor_right(out: &Term, n: usize) -> io::Result<()> {
    if n > 0 {
        out.write_str(&format!("\x1b[{}C", n))
    } else {
        Ok(())
    }
}

#[inline]
pub fn move_cursor_to(out: &Term, x: usize, y: usize) -> io::Result<()> {
    out.write_str(&format!("\x1B[{};{}H", y + 1, x + 1))
}

#[cfg(unix)]
/// Return the current cursor's position as a tuple `(n, m)`,
/// where `n` is the row and `m` the column of the cursor (both 1-based).
pub fn get_cursor_position(mut out: &Term) -> io::Result<(u16, u16)> {
    // Send the code "ESC[6n" to the terminal: asking for the current cursor position.
    out.write_str("\x1b[6n")?;
    // We expect a response of the form "ESC[n;mR", where n and m are the row and column of the cursor.
    // n and m are at most 65536, so 4+2*5 bytes should suffice for these purposes.
    // TODO: this blocks for user input!
    let mut buf = [0u8; 4 + 2 * 5];
    let num_read = io::Read::read(&mut out, &mut buf)?;
    out.clear_chars(num_read)?;
    // FIXME: re-use ANSI code parser instead of rolling my own.
    match &buf[..] {
        [b'\x1B', b'[', middle @ .., b'R'] => {
            // A valid terminal response means `middle` is valid UTF-8.
            // Use string methods to simplify the parsing of input.
            let middle = match std::str::from_utf8(middle) {
                Ok(m) => m,
                Err(_) => return Err(io::Error::new(io::ErrorKind::Other, format!("invalid terminal response: middle part of the output {:?} must be valid UTF-8", buf))),
            };
            let parts = middle.splitn(2, ';').collect::<Vec<_>>();
            let (nstr, mstr) =
            match &parts[..] {
                [a, b] => (a, b),
                _ => return Err(io::Error::new(io::ErrorKind::Other, format!("invalid terminal response: middle part of the output should be of the form n;m, got {}", middle))),
            };
            let (n, m) = (nstr.parse::<u16>().unwrap(), mstr.parse::<u16>().unwrap());
            Ok((n, m))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::Other,
            "invalid terminal response: should be of the form ESC[n;mR",
        )),
    }
}

pub fn clear_chars(out: &Term, n: usize) -> io::Result<()> {
    if n > 0 {
        out.write_str(&format!("\x1b[{}D\x1b[0K", n))
    } else {
        Ok(())
    }
}

#[inline]
pub fn clear_line(out: &Term) -> io::Result<()> {
    out.write_str("\r\x1b[2K")
}

#[inline]
pub fn clear_screen(out: &Term) -> io::Result<()> {
    out.write_str("\r\x1b[2J\r\x1b[H")
}

#[inline]
pub fn clear_to_end_of_screen(out: &Term) -> io::Result<()> {
    out.write_str("\r\x1b[0J")
}

#[inline]
pub fn show_cursor(out: &Term) -> io::Result<()> {
    out.write_str("\x1b[?25h")
}

#[inline]
pub fn hide_cursor(out: &Term) -> io::Result<()> {
    out.write_str("\x1b[?25l")
}
