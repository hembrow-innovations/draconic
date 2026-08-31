//! ROADMAP U12: extract `/** … */` doc comments and emit markdown or HTML.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocItem {
    pub name: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocFormat {
    Markdown,
    Html,
}

impl DocFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "md" | "markdown" => Some(Self::Markdown),
            "html" | "htm" => Some(Self::Html),
            _ => None,
        }
    }
}

/// Extract `/** … */` comments attached to the next declaration name.
pub fn extract_docs(source: &str) -> Vec<DocItem> {
    let bytes = source.as_bytes();
    let mut items = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        // Skip line comments.
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Block comment: `/* … */` — doc only when `/**` and not `/***`.
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            let is_doc = i + 2 < bytes.len()
                && bytes[i + 2] == b'*'
                && !(i + 3 < bytes.len() && bytes[i + 3] == b'/');
            let start = i;
            i += 2;
            if is_doc {
                i += 1; // skip the third `*`
            }
            let body_start = i;
            while i + 1 < bytes.len() {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    break;
                }
                i += 1;
            }
            if i + 1 >= bytes.len() {
                break;
            }
            let body_end = i;
            i += 2; // */
            if !is_doc {
                continue;
            }
            let raw = &source[body_start..body_end];
            let body = clean_doc_body(raw);
            if body.is_empty() {
                continue;
            }
            if let Some(name) = next_decl_name(&source[i..]) {
                let _ = start;
                items.push(DocItem { name, body });
            }
            continue;
        }

        // Skip strings so `/*` inside them is not treated as a comment.
        if bytes[i] == b'\'' || bytes[i] == b'"' || bytes[i] == b'`' {
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                    continue;
                }
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                // Template `${` — naive skip until matching `}` depth (best-effort).
                if quote == b'`' && bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{'
                {
                    i += 2;
                    let mut depth = 1i32;
                    while i < bytes.len() && depth > 0 {
                        if bytes[i] == b'{' {
                            depth += 1;
                        } else if bytes[i] == b'}' {
                            depth -= 1;
                        } else if bytes[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    continue;
                }
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    items
}

fn clean_doc_body(raw: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in raw.lines() {
        let mut s = line.trim_end();
        // Strip common leading indent + optional `*`.
        let trimmed = s.trim_start();
        if let Some(rest) = trimmed.strip_prefix('*') {
            s = rest.strip_prefix(' ').unwrap_or(rest);
        } else {
            s = trimmed;
        }
        lines.push(s.to_string());
    }
    // Trim leading/trailing blank lines.
    while lines.first().is_some_and(|l| l.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

fn next_decl_name(after: &str) -> Option<String> {
    let s = after.trim_start();
    let s = strip_export(s);
    let s = s.trim_start();

    // async function[*] name
    // function[*] name
    // class name
    // const|let|var name
    let s = if let Some(rest) = strip_keyword(s, "async") {
        rest.trim_start()
    } else {
        s
    };

    if let Some(rest) = strip_keyword(s, "function") {
        let rest = rest.trim_start();
        let rest = rest.strip_prefix('*').unwrap_or(rest).trim_start();
        return ident_at(rest);
    }
    if let Some(rest) = strip_keyword(s, "class") {
        return ident_at(rest.trim_start());
    }
    for kw in ["const", "let", "var"] {
        if let Some(rest) = strip_keyword(s, kw) {
            return ident_at(rest.trim_start());
        }
    }
    None
}

fn strip_export(s: &str) -> &str {
    if let Some(rest) = strip_keyword(s, "export") {
        let rest = rest.trim_start();
        if let Some(rest) = strip_keyword(rest, "default") {
            rest
        } else {
            rest
        }
    } else {
        s
    }
}

fn strip_keyword<'a>(s: &'a str, kw: &str) -> Option<&'a str> {
    if s.starts_with(kw) {
        let rest = &s[kw.len()..];
        if rest.is_empty() || rest.chars().next().is_some_and(|c| !is_ident_continue(c)) {
            return Some(rest);
        }
    }
    None
}

fn ident_at(s: &str) -> Option<String> {
    let mut chars = s.chars();
    let first = chars.next()?;
    if !is_ident_start(first) {
        return None;
    }
    let mut name = String::new();
    name.push(first);
    for c in chars {
        if is_ident_continue(c) {
            name.push(c);
        } else {
            break;
        }
    }
    Some(name)
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c == '$' || c.is_ascii_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}

pub fn render_markdown(title: &str, items: &[DocItem]) -> String {
    let mut out = String::new();
    out.push_str("# ");
    out.push_str(title);
    out.push('\n');
    for item in items {
        out.push('\n');
        out.push_str("## `");
        out.push_str(&item.name);
        out.push_str("`\n\n");
        out.push_str(&item.body);
        out.push('\n');
    }
    out
}

pub fn render_html(title: &str, items: &[DocItem]) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str("<title>");
    out.push_str(&escape_html(title));
    out.push_str("</title>\n</head>\n<body>\n");
    out.push_str("<h1>");
    out.push_str(&escape_html(title));
    out.push_str("</h1>\n");
    for item in items {
        out.push_str("<section id=\"");
        out.push_str(&escape_html(&item.name));
        out.push_str("\">\n<h2><code>");
        out.push_str(&escape_html(&item.name));
        out.push_str("</code></h2>\n");
        for para in item.body.split("\n\n") {
            out.push_str("<p>");
            out.push_str(&escape_html(para).replace('\n', "<br>\n"));
            out.push_str("</p>\n");
        }
        out.push_str("</section>\n");
    }
    out.push_str("</body>\n</html>\n");
    out
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_function_and_class() {
        let src = r#"
/**
 * Hello.
 */
function foo() {}

/** Bar */
class Bar {}
"#;
        let items = extract_docs(src);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "foo");
        assert_eq!(items[0].body, "Hello.");
        assert_eq!(items[1].name, "Bar");
        assert_eq!(items[1].body, "Bar");
    }

    #[test]
    fn ignores_non_doc_block_comments() {
        let src = "/* not doc */\nfunction foo() {}\n";
        assert!(extract_docs(src).is_empty());
    }

    #[test]
    fn export_async() {
        let src = "/** Pub */\nexport async function pub() {}\n";
        let items = extract_docs(src);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "pub");
    }
}
