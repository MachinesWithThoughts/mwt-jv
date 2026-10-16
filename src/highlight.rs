use ratatui::{
    style::Color,
    text::{Line, Span},
};

pub const KEY: Color = Color::Rgb(125, 190, 255);
pub const STRING: Color = Color::Rgb(167, 218, 151);
pub const NUMBER: Color = Color::Rgb(245, 190, 120);
pub const BOOL: Color = Color::Rgb(198, 160, 246);
pub const MUTED: Color = Color::Rgb(123, 139, 161);
pub const TEXT: Color = Color::Rgb(215, 225, 240);

/// Tokenize serialized JSON, respecting escaped quotes and Unicode boundaries.
pub fn json(text: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut tokens = text.char_indices().peekable();
    while let Some((start, ch)) = tokens.next() {
        let mut end = start + ch.len_utf8();
        let color = if ch == '"' {
            let mut escaped = false;
            for (pos, c) in tokens.by_ref() {
                end = pos + c.len_utf8();
                if c == '"' && !escaped {
                    break;
                }
                escaped = c == '\\' && !escaped;
            }
            if text[end..].trim_start().starts_with(':') {
                KEY
            } else {
                STRING
            }
        } else if ch.is_ascii_digit() || ch == '-' {
            while let Some(&(pos, c)) = tokens.peek() {
                if !(c.is_ascii_digit() || ".eE+-".contains(c)) {
                    break;
                }
                tokens.next();
                end = pos + c.len_utf8();
            }
            NUMBER
        } else if ch.is_ascii_alphabetic() {
            while let Some(&(pos, c)) = tokens.peek() {
                if !c.is_ascii_alphabetic() {
                    break;
                }
                tokens.next();
                end = pos + c.len_utf8();
            }
            if &text[start..end] == "null" {
                MUTED
            } else {
                BOOL
            }
        } else {
            TEXT
        };
        spans.push(Span::styled(text[start..end].to_string(), color));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escaped_quotes_and_unicode_are_preserved() {
        let input = r#"{"雪": "a\"b\\", "n": -1.2e+3, "v": null}"#;
        let line = json(input);
        assert_eq!(line.to_string(), input);
        assert_eq!(line.spans[1].style.fg, Some(KEY));
        assert!(
            line.spans
                .iter()
                .any(|s| s.style.fg == Some(NUMBER) && s.content == "-1.2e+3")
        );
    }
}
