use std::io;
use std::io::Write;
use std::sync::Arc;

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, RawHandle};

use kb::Key;

use clicolors_control;
use parking_lot::Mutex;

/// Where the term is writing.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TermTarget {
    Stdout,
    Stderr,
}

#[derive(Debug)]
pub struct TermInner {
    target: TermTarget,
    buffer: Option<Mutex<Vec<u8>>>,
}

/// The family of the terminal.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TermFamily {
    /// Redirected to a file or file like thing.
    File,
    /// A standard unix terminal.
    UnixTerm,
    /// A cmd.exe like windows console.
    WindowsConsole,
}

/// Gives access to the terminal features.
#[derive(Debug, Clone)]
pub struct TermFeatures<'a>(&'a Term);

impl<'a> TermFeatures<'a> {
    /// Checks if this is a real user attended terminal (`isatty`)
    #[inline]
    pub fn is_attended(&self) -> bool {
        is_a_terminal(self.0)
    }

    /// Checks if colors are supported by this terminal.
    ///
    /// This does not check if colors are enabled.  Currently all terminals
    /// are considered to support colors
    #[inline]
    pub fn colors_supported(&self) -> bool {
        clicolors_control::terminfo::supports_colors()
    }

    /// Checks if this terminal is an msys terminal.
    ///
    /// This is sometimes useful to disable features that are known to not
    /// work on msys terminals or require special handling.
    #[inline]
    pub fn is_msys_tty(&self) -> bool {
        #[cfg(windows)]
        {
            msys_tty_on(&self.0)
        }
        #[cfg(unix)]
        {
            false
        }
    }

    /// Checks if this terminal wants emojis.
    #[inline]
    pub fn wants_emoji(&self) -> bool {
        self.is_attended() && wants_emoji()
    }

    /// Returns the family of the terminal.
    #[inline]
    pub fn family(&self) -> TermFamily {
        if !self.is_attended() {
            return TermFamily::File;
        }
        #[cfg(windows)]
        {
            TermFamily::WindowsConsole
        }
        #[cfg(unix)]
        {
            TermFamily::UnixTerm
        }
    }
}

/// Abstraction around a terminal.
///
/// A terminal can be cloned.  If a buffer is used it's shared across all
/// clones which means it largely acts as a handle.
#[derive(Clone, Debug)]
pub struct Term {
    inner: Arc<TermInner>,
}

impl Term {
    fn with_inner(inner: TermInner) -> Term {
        Term {
            inner: Arc::new(inner),
        }
    }

    /// Return a new unbuffered terminal
    #[inline]
    pub fn stdout() -> Term {
        Term::with_inner(TermInner {
            target: TermTarget::Stdout,
            buffer: None,
        })
    }

    /// Return a new unbuffered terminal to stderr
    #[inline]
    pub fn stderr() -> Term {
        Term::with_inner(TermInner {
            target: TermTarget::Stderr,
            buffer: None,
        })
    }

    /// Return a new buffered terminal
    pub fn buffered_stdout() -> Term {
        Term::with_inner(TermInner {
            target: TermTarget::Stdout,
            buffer: Some(Mutex::new(vec![])),
        })
    }

    /// Return a new buffered terminal to stderr
    pub fn buffered_stderr() -> Term {
        Term::with_inner(TermInner {
            target: TermTarget::Stderr,
            buffer: Some(Mutex::new(vec![])),
        })
    }
    /// Returns the targert
    pub fn target(&self) -> TermTarget {
        self.inner.target
    }

    #[doc(hidden)]
    pub fn write_str(&self, s: &str) -> io::Result<()> {
        match self.inner.buffer {
            Some(ref buffer) => buffer.lock().write_all(s.as_bytes()),
            None => self.write_through(s.as_bytes()),
        }
    }

    /// Writes a string to the terminal and adds a newline.
    pub fn write_line(&self, s: &str) -> io::Result<()> {
        match self.inner.buffer {
            Some(ref mutex) => {
                let mut buffer = mutex.lock();
                buffer.extend_from_slice(s.as_bytes());
                buffer.push(b'\n');
                Ok(())
            }
            None => self.write_through(format!("{}\n", s).as_bytes()),
        }
    }

    /// Read a single character from the terminal
    ///
    /// This does not echo the character and blocks until a single character
    /// is entered.
    pub fn read_char(&self) -> io::Result<char> {
        loop {
            match self.read_key()? {
                Key::Char(c) => {
                    return Ok(c);
                }
                Key::Enter => {
                    return Ok('\n');
                }
                _ => {}
            }
        }
    }

    /// Read a single key form the terminal.
    ///
    /// This does not echo anything.  If the terminal is not user attended
    /// the return value will always be the unknown key.
    pub fn read_key(&self) -> io::Result<Key> {
        if !self.is_term() {
            Ok(Key::Unknown)
        } else {
            read_single_key()
        }
    }

    /// Read one line of input.
    ///
    /// This does not include the trailing newline.  If the terminal is not
    /// user attended the return value will always be an empty string.
    pub fn read_line(&self) -> io::Result<String> {
        if !self.is_term() {
            return Ok("".into());
        }
        let mut rv = String::new();
        io::stdin().read_line(&mut rv)?;
        let len = rv.trim_right_matches(&['\r', '\n'][..]).len();
        rv.truncate(len);
        Ok(rv)
    }

    /// Read securely a line of input.
    ///
    /// This is similar to `read_line` but will not echo the output.  This
    /// also switches the terminal into a different mode where not all
    /// characters might be accepted.
    pub fn read_secure_line(&self) -> io::Result<String> {
        if !self.is_term() {
            return Ok("".into());
        }
        match read_secure() {
            Ok(rv) => {
                self.write_line("")?;
                Ok(rv)
            }
            Err(err) => Err(err),
        }
    }

    /// Flushes internal buffers.
    ///
    /// This forces the contents of the internal buffer to be written to
    /// the terminal.  This is unnecessary for unbuffered terminals which
    /// will automatically flush.
    pub fn flush(&self) -> io::Result<()> {
        if let Some(ref buffer) = self.inner.buffer {
            let mut buffer = buffer.lock();
            if !buffer.is_empty() {
                self.write_through(&buffer[..])?;
                buffer.clear();
            }
        }
        Ok(())
    }

    /// Checks if the terminal is indeed a terminal.
    ///
    /// This is a shortcut for `features().is_attended()`.
    pub fn is_term(&self) -> bool {
        self.features().is_attended()
    }

    /// Checks for common terminal features.
    #[inline]
    pub fn features(&self) -> TermFeatures<'_> {
        TermFeatures(self)
    }

    /// Checks if this terminal wants emoji output.
    #[deprecated(note = "Use features().wants_emoji() instead", since = "0.8.0")]
    pub fn want_emoji(&self) -> bool {
        self.features().wants_emoji()
    }

    /// Returns the terminal size or gets sensible defaults.
    #[inline]
    pub fn size(&self) -> (u16, u16) {
        self.size_checked().unwrap_or((24, DEFAULT_WIDTH))
    }

    /// Returns the terminal size in rows and columns.
    ///
    /// If the size cannot be reliably determined None is returned.
    #[inline]
    pub fn size_checked(&self) -> Option<(u16, u16)> {
        terminal_size()
    }

    /// Moves the cursor up `n` lines
    pub fn move_cursor_up(&self, n: usize) -> io::Result<()> {
        move_cursor_up(self, n)
    }

    /// Moves the cursor down `n` lines
    pub fn move_cursor_down(&self, n: usize) -> io::Result<()> {
        move_cursor_down(self, n)
    }

    /// Clears the current line.
    ///
    /// The positions the cursor at the beginning of the line again.
    pub fn clear_line(&self) -> io::Result<()> {
        clear_line(self)
    }

    /// Clear the last `n` lines.
    ///
    /// This positions the cursor at the beginning of the first line
    /// that was cleared.
    pub fn clear_last_lines(&self, n: usize) -> io::Result<()> {
        self.move_cursor_up(n)?;
        for _ in 0..n {
            self.clear_line()?;
            self.move_cursor_down(1)?;
        }
        self.move_cursor_up(n)?;
        Ok(())
    }

    /// Clears the entire screen.
    pub fn clear_screen(&self) -> io::Result<()> {
        clear_screen(self)
    }

    // helpers

    fn write_through(&self, bytes: &[u8]) -> io::Result<()> {
        match self.inner.target {
            TermTarget::Stdout => {
                io::stdout().write_all(bytes)?;
                io::stdout().flush()?;
            }
            TermTarget::Stderr => {
                io::stderr().write_all(bytes)?;
                io::stderr().flush()?;
            }
        }
        Ok(())
    }
}

/// A fast way to check if the application has a user attended.
///
/// This means that stdout is connected to a terminal instead of a
/// file or redirected by other means.  This is a shortcut for
/// checking the `is_attended` flag on the stdout terminal.
pub fn user_attended() -> bool {
    Term::stdout().features().is_attended()
}

#[cfg(unix)]
impl AsRawFd for Term {
    fn as_raw_fd(&self) -> RawFd {
        use libc;
        match self.inner.target {
            TermTarget::Stdout => libc::STDOUT_FILENO,
            TermTarget::Stderr => libc::STDERR_FILENO,
        }
    }
}

#[cfg(windows)]
impl AsRawHandle for Term {
    fn as_raw_handle(&self) -> RawHandle {
        use winapi::um::processenv::GetStdHandle;
        use winapi::um::winbase::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};

        unsafe {
            GetStdHandle(match self.inner.target {
                TermTarget::Stdout => STD_OUTPUT_HANDLE,
                TermTarget::Stderr => STD_ERROR_HANDLE,
            }) as RawHandle
        }
    }
}

impl io::Write for Term {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_through(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl io::Read for Term {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        io::stdin().read(buf)
    }
}

#[cfg(unix)]
pub use unix_term::*;
#[cfg(windows)]
pub use windows_term::*;
