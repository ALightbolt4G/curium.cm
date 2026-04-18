use ariadne::{Color, Label, Report, ReportKind, Source};

/// Emit a rich parse error with source context.
pub fn emit_parse_error(source: &str, filename: &str, offset: usize, message: &str) {
    Report::build(ReportKind::Error, filename, offset)
        .with_message(message)
        .with_label(
            Label::new((filename, offset..offset + 1))
                .with_message(message)
                .with_color(Color::Red),
        )
        .finish()
        .eprint((filename, Source::from(source)))
        .ok();
}

/// Emit a type mismatch error.
#[allow(dead_code)]
pub fn emit_type_error(
    source: &str,
    filename: &str,
    offset: usize,
    end: usize,
    expected: &str,
    found: &str,
) {
    Report::build(ReportKind::Error, filename, offset)
        .with_message(format!("Type mismatch: expected {}, found {}", expected, found))
        .with_label(
            Label::new((filename, offset..end))
                .with_message(format!("this has type {}", found))
                .with_color(Color::Red),
        )
        .with_label(
            Label::new((filename, offset..end))
                .with_message(format!("expected {}", expected))
                .with_color(Color::Blue),
        )
        .finish()
        .eprint((filename, Source::from(source)))
        .ok();
}

/// Emit a warning.
#[allow(dead_code)]
pub fn emit_warning(source: &str, filename: &str, offset: usize, end: usize, message: &str) {
    Report::build(ReportKind::Warning, filename, offset)
        .with_message(message)
        .with_label(
            Label::new((filename, offset..end))
                .with_message(message)
                .with_color(Color::Yellow),
        )
        .finish()
        .eprint((filename, Source::from(source)))
        .ok();
}

/// Format a simple error string with line/column info.
pub fn format_error(filename: &str, line: usize, col: usize, msg: &str) -> String {
    format!(
        "\x1b[1;31merror\x1b[0m: {}\n  \x1b[1;34m-->\x1b[0m {}:{}:{}",
        msg, filename, line, col
    )
}
