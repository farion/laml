//! Escape text and identifier attribute values before SVG serialization.
pub(super) fn escape(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(ch),
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn escapes_untrusted_labels() {
        assert_eq!(escape("<&\"'"), "&lt;&amp;&quot;&apos;");
    }
}
