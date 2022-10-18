use std::io;

use console::Term;

fn main() -> io::Result<()> {
    let term = Term::stdout();
    let (height, width) = term.size();
    term.hide_cursor()?;
    term.enter_alternate_screen()?;

    let s = "press any key to quit";
    let x = (width as usize / 2) - (s.len() / 2);
    let y = height as usize / 2;
    term.move_cursor_to(x, y)?;
    term.write_str(&s)?;
    term.read_key()?;

    term.exit_alternate_screen()?;
    term.show_cursor()?;
    Ok(())
}
