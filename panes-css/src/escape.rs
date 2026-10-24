use std::fmt::{self, Display, Write};

/// Write `[attr="value"]`, escaping the value as a CSS string.
pub(crate) fn write_attr_selector(
    attr: &str,
    value: impl Display,
    out: &mut impl Write,
) -> fmt::Result {
    write!(out, "[{attr}=\"")?;
    write!(CssEscape(&mut *out), "{value}")?;
    out.write_str("\"]")
}

/// Writer adapter that escapes text for a double-quoted CSS string.
struct CssEscape<W>(W);

impl<W: Write> Write for CssEscape<W> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        value.chars().try_for_each(|ch| self.write_char(ch))
    }

    fn write_char(&mut self, ch: char) -> fmt::Result {
        match ch {
            '\\' | '"' => {
                self.0.write_char('\\')?;
                self.0.write_char(ch)
            }
            '\0' => self.0.write_str("\\0 "),
            '\n' => self.0.write_str("\\a "),
            '\r' => self.0.write_str("\\d "),
            '\u{000C}' => self.0.write_str("\\c "),
            _ => self.0.write_char(ch),
        }
    }
}
