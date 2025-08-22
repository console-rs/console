extern crate console;
use console::Term;
use std::io;

fn draw_point(term: &Term, width: usize) -> io::Result<()> {
    // no cheating...get the cursor position here
    let (x, y) = term.cursor_position()?;
    let str = format!("({x}, {y})");
    let w = str.len() + 2;
    if x + w > width {
        term.move_cursor_left(w - 1)?;
        term.write_str(&format!("{str} •"))?;
    } else {
        term.write_str(&format!("• {str}"))?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let term = Term::stdout();
    term.hide_cursor()?;
    term.clear_screen()?;

    let (height, width): (usize, usize) = (term.size().0 as usize, term.size().1 as usize);

    // draw the four corners
    term.move_cursor_to(0, 0)?;
    draw_point(&term, width)?;
    // this tests the formatting logic
    for i in 0..20 {
        term.move_cursor_to(width - i - 1, i)?;
        draw_point(&term, width)?;
    }
    term.move_cursor_to(0, height - 2)?;
    draw_point(&term, width)?;
    term.move_cursor_to(width, height - 2)?;
    draw_point(&term, width)?;

    for _ in 0..10 {
        let x = rand::random_range(..=width - 1);
        let y = rand::random_range(1..=height - 3);
        term.move_cursor_to(x, y)?;
        draw_point(&term, width)?;
    }

    term.move_cursor_to(0, height)?;
    term.show_cursor()?;

    Ok(())
}
