use console::{style, Color, Style};

fn main() {
    // Basic foreground gradient using the builder API
    println!(
        "Foreground gradient: {}",
        style("Hello, gradient world!")
            .fg(Color::TrueColor(255, 0, 0))
            .fg_end(Color::TrueColor(0, 0, 255))
    );

    // Background gradient
    println!(
        "Background gradient: {}",
        style("  Gradient background  ")
            .bg(Color::TrueColor(255, 0, 0))
            .bg_end(Color::TrueColor(54, 36, 79))
    );

    // Both foreground and background gradients
    println!(
        "Foreground and background gradients: {}",
        style("Fancy text!")
            .fg(Color::TrueColor(255, 255, 0))
            .fg_end(Color::TrueColor(255, 0, 255))
            .bg(Color::TrueColor(0, 0, 128))
            .bg_end(Color::TrueColor(0, 128, 0))
    );

    // Using from_dotted_str for gradient specification
    let gradient_style = Style::from_dotted_str("#ff0000->#00ff00.bold");
    println!(
        "From dotted string: {}",
        gradient_style.apply_to("Styled from_dotted_str")
    );

    // Gradient with named colors (they get converted to RGB for interpolation)
    println!(
        "Named color gradient: {}",
        style("Red to Blue").fg(Color::Red).fg_end(Color::Blue)
    );

    // Progress bar simulation
    let width = 40;
    let progress = 0.7; // 70% complete
    let filled = (width as f32 * progress) as usize;
    let bar: String = "█".repeat(filled) + &"░".repeat(width - filled);
    println!(
        "Progress: [{}] {}%",
        style(&bar)
            .fg(Color::TrueColor(0, 255, 0))
            .fg_end(Color::TrueColor(0, 128, 255)),
        (progress * 100.0) as u32
    );
}
