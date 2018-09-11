extern crate console;

use std::io;
use std::thread;
use std::time::Duration;

use console::{style, Term};

fn do_stuff() -> io::Result<()> {
    let term = Term::stdout();
    term.write_line("Going to do some counting now")?;
    for x in 0..10 {
        if x != 0 {
            term.move_cursor_up(1)?;
        }
        term.write_line(&format!("Counting {}/10", style(x + 1).cyan()))?;
        thread::sleep(Duration::from_millis(400));
    }
    term.clear_last_lines(1)?;
    term.write_line("Done counting!")?;
    Ok(())
}

fn main() {
    do_stuff().unwrap();
}
