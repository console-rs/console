#![cfg(all(feature = "std", any(target_os = "linux", target_os = "macos")))]

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct TerminalChild(Child);

impl Drop for TerminalChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn terminal_pair() -> io::Result<(File, File)> {
    let mut master = -1;
    let mut slave = -1;
    // SAFETY: openpty initializes the two descriptors; no optional outputs are requested.
    if unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openpty returned newly owned file descriptors.
    let (master, slave) = unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) };
    let mut termios = std::mem::MaybeUninit::uninit();
    // SAFETY: slave is open and termios points to writable storage.
    if unsafe { libc::tcgetattr(slave.as_raw_fd(), termios.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: tcgetattr successfully initialized termios.
    let mut termios = unsafe { termios.assume_init() };
    // Disable canonical editing so queued Backspace keys reach console unchanged.
    // SAFETY: termios is initialized and slave is an open terminal descriptor.
    unsafe {
        libc::cfmakeraw(&mut termios);
        if libc::tcsetattr(slave.as_raw_fd(), libc::TCSANOW, &termios) != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok((master, slave))
}

#[test]
fn read_line_initial_text_preserves_input() {
    const PREFIX: &str = "CONSOLE_TEST_PREFIX";
    const EXPECTED: &str = "CONSOLE_TEST_EXPECTED";
    if let Ok(prefix) = std::env::var(PREFIX) {
        let actual = console::Term::stdout()
            .read_line_initial_text(&prefix)
            .unwrap();
        assert_eq!(actual, std::env::var(EXPECTED).unwrap());
        return;
    }

    for (prefix, input, expected) in [
        ("> ", "abc\r", "abc"),
        ("λ", "abc\r", "abc"),
        ("🦀", "abc\r", "abc"),
        ("日本語", "abc\r", "abc"),
        ("e\u{301}", "λ🦀\r", "λ🦀"),
        ("", "λ🦀\r", "λ🦀"),
        ("🦀", "\x7fa\x7fλ\x7fZ\r", "Z"),
        ("日本語", "\x7f\r", ""),
    ] {
        let (mut master, slave) = terminal_pair().unwrap();
        let mut child = TerminalChild(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "read_line_initial_text_preserves_input",
                    "--nocapture",
                ])
                .env(PREFIX, prefix)
                .env(EXPECTED, expected)
                .stdin(Stdio::from(slave.try_clone().unwrap()))
                .stdout(Stdio::from(slave.try_clone().unwrap()))
                .stderr(Stdio::from(slave))
                .spawn()
                .unwrap(),
        );
        // Drain output concurrently: read_key uses TCSADRAIN when changing terminal modes.
        let mut reader = master.try_clone().unwrap();
        let output = thread::spawn(move || {
            let mut output = Vec::new();
            match reader.read_to_end(&mut output) {
                Ok(_) => (),
                // Linux signals the closed slave with EIO rather than EOF.
                Err(err) if err.raw_os_error() == Some(libc::EIO) => (),
                Err(err) => panic!("reading terminal output: {err}"),
            }
            output
        });
        master.write_all(input.as_bytes()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.0.kill().unwrap();
                break child.0.wait().unwrap();
            }
            thread::sleep(Duration::from_millis(10));
        };
        let output = output.join().unwrap();
        assert!(
            status.success(),
            "prefix {prefix:?}, input {input:?}: {}",
            String::from_utf8_lossy(&output)
        );
    }
}
