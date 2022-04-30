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

/// Return the current cursor's position as a tuple `(n, m)`,
/// where `n` is the row and `m` the column of the cursor (both 1-based).
// FIXME: allow a larger range of characters than u8.
// FIXME: clear the terminal after this operation.
pub fn get_cursor_position(mut out: &Term) -> io::Result<(u8, u8)> {
    // Send the code ESC6n to the terminal: asking for the current cursor position.
    out.write_str("\x1b[6n")?;
    // We expect a response ESC[n;mR, where n and m are the row and column of the cursor.
    let mut buf = [0u8; 6];
    let num_read = io::Read::read(&mut out, &mut buf)?;
    let (n, m) = match &buf[..] {
        // If we didn't read enough bytes, we certainly didn't get the response we wanted.
        _ if num_read < buf.len() => return Err(std::io::Error::new(
            io::ErrorKind::Other, format!("invalid terminal response: expected six bytes, only read {}", num_read)
        )),
        [a, b, n, c, m, d] => {
            // The bytes a, b, c and d should be byte string \x1 [ ; R.
            if &[*a, *b, *c, *d] != b"\x1b[;R" {
                return Err(std::io::Error::new(io::ErrorKind::Other, "invalid terminal response: should be of the form ESC[n;mR"));
            } else {
                (n, m)
            }
        }
        _ => unreachable!(),
    };
    Ok((*n, *m))
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
