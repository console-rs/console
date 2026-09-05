#![cfg(all(feature = "std", feature = "ansi-parsing", feature = "unicode-width"))]

use console::measure_text_width;

#[test]
fn printable_ascii_uses_width() {
    assert_eq!(measure_text_width(""), 0);
    assert_eq!(measure_text_width(" !~"), 3);
}

#[test]
fn controls_and_ansi_fall_back_to_parser() {
    // Existing str_width counts the newline as one column in this contract.
    assert_eq!(measure_text_width("a\nb"), 3);
    assert_eq!(measure_text_width("\x1b[31mred\x1b[0m"), 3);
    assert_eq!(measure_text_width("\u{9b}31mred\u{9b}0m"), 3);
}

#[test]
fn unicode_width_falls_back() {
    assert_eq!(measure_text_width("é"), 1);
    assert_eq!(measure_text_width("e\u{301}"), 1);
    assert_eq!(measure_text_width("1\u{fe0f}\u{20e3}"), 2);
    assert_eq!(measure_text_width("👩‍💻"), 2);
}

#[test]
fn long_ascii_prefix_with_later_control_falls_back() {
    let mut value = "x".repeat(4096);
    value.push('\n');
    value.push('y');
    assert_eq!(measure_text_width(&value), 4098);
}
