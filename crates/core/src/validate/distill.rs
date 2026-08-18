//! context-distill — deterministic HTML-to-clean-text distillation.
//!
//! Day-1 nectar product (docs/NECTAR-STRATEGY.md): agents send raw HTML,
//! receive dense typed text optimized for LLM context windows.
//!
//! HARD INVARIANTS:
//! - DETERMINISTIC: identical input yields byte-identical output. No clock,
//!   no randomness, no network. The client supplies the HTML — the tool never
//!   fetches URLs (no SSRF surface, no legality exposure).
//! - ZERO-ALLOCATION-TRIM where practical; output bounded by max_bytes.

/// Decode the small set of entities that matter for text extraction.
fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}

/// Remove a paired tag and its entire contents, case-insensitively
/// (e.g. script, style). Returns the input with all such blocks removed.
fn strip_block_tags(html: &str, tag: &str) -> String {
    let lower = html.to_lowercase();
    let open_pat = format!("<{tag}");
    let close_pat = format!("</{tag}>");
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;

    while let Some(open_rel) = lower[cursor..].find(&open_pat) {
        let open_abs = cursor + open_rel;
        // Find the end of the opening tag '>'
        let Some(gt_rel) = lower[open_abs..].find('>') else {
            // Malformed: emit rest verbatim and stop
            out.push_str(&html[cursor..]);
            return out;
        };
        let content_start = open_abs + gt_rel + 1;
        // Emit everything before the block
        out.push_str(&html[cursor..open_abs]);
        // Skip to after the closing tag
        match lower[content_start..].find(&close_pat) {
            Some(close_rel) => {
                cursor = content_start + close_rel + close_pat.len();
            }
            None => {
                // Unclosed block: drop everything to EOF (fail-closed on scripts)
                return out;
            }
        }
    }
    out.push_str(&html[cursor..]);
    out
}

/// Remove HTML comments <!-- ... -->.
fn strip_comments(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;
    while let Some(open_rel) = html[cursor..].find("<!--") {
        let open_abs = cursor + open_rel;
        out.push_str(&html[cursor..open_abs]);
        match html[open_abs..].find("-->") {
            Some(close_rel) => cursor = open_abs + close_rel + 3,
            None => return out, // unclosed comment: drop to EOF
        }
    }
    out.push_str(&html[cursor..]);
    out
}

/// Remove all markup tags, keeping text content.
/// Tags act as word separators (removed markup ⇒ single space, collapsed later).
fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Collapse all whitespace runs to a single space and trim.
fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut pending_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            pending_space = true;
        } else {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(c);
        }
    }
    out
}

/// Truncate at a word boundary, not exceeding max_bytes (UTF-8 safe).
fn truncate_at_word_boundary(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    // Walk back to the last space to avoid cutting a word
    match s[..end].rfind(' ') {
        Some(sp) if sp > max_bytes / 2 => s[..sp].to_string(),
        _ => s[..end].to_string(),
    }
}

pub struct DistillResult {
    pub clean_text: String,
    pub original_bytes: usize,
    pub output_bytes: usize,
    pub estimated_tokens_saved: usize,
}

/// The deterministic distillation pipeline.
pub fn distill(html: &str, max_bytes: usize) -> DistillResult {
    let original_bytes = html.len();
    let no_scripts = strip_block_tags(html, "script");
    let no_styles = strip_block_tags(&no_scripts, "style");
    let no_comments = strip_comments(&no_styles);
    let no_tags = strip_tags(&no_comments);
    let decoded = decode_entities(&no_tags);
    let collapsed = collapse_whitespace(&decoded);
    let clean_text = truncate_at_word_boundary(&collapsed, max_bytes);
    let output_bytes = clean_text.len();
    DistillResult {
        clean_text,
        original_bytes,
        output_bytes,
        estimated_tokens_saved: original_bytes.saturating_sub(output_bytes) / 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_script_style_and_tags() {
        let html = r#"<html><head><style>body{color:red}</style><script>alert(1)</script></head><body><h1>Hello</h1><p>World</p></body></html>"#;
        let r = distill(html, 4000);
        assert_eq!(r.clean_text, "Hello World");
    }

    #[test]
    fn deterministic_byte_identical() {
        let html = "<p>A</p><script>x</script><p>B</p>";
        assert_eq!(distill(html, 4000).clean_text, distill(html, 4000).clean_text);
    }

    #[test]
    fn collapses_whitespace_and_decodes_entities() {
        let html = "<p>Fish   &amp;\n\t Chips&nbsp;&lt;3</p>";
        let r = distill(html, 4000);
        assert_eq!(r.clean_text, "Fish & Chips <3");
    }

    #[test]
    fn strips_comments() {
        let html = "<p>A</p><!-- hidden --><p>B</p>";
        assert_eq!(distill(html, 4000).clean_text, "A B");
    }

    #[test]
    fn truncates_at_word_boundary() {
        let html = format!("<p>{}</p>", "word ".repeat(2000));
        let r = distill(&html, 100);
        assert!(r.output_bytes <= 100);
        assert!(!r.clean_text.ends_with("wo")); // not mid-word
    }

    #[test]
    fn unclosed_script_fails_closed() {
        let html = "<p>safe</p><script>malicious(";
        let r = distill(html, 4000);
        assert_eq!(r.clean_text, "safe");
    }

    #[test]
    fn token_savings_estimated() {
        let html = format!("<div>{}</div>", "x".repeat(400));
        let r = distill(&html, 4000);
        assert!(r.estimated_tokens_saved > 0);
    }
}
