extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub tag: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn new(tag: &str) -> Self {
        Self {
            tag: String::from(tag),
            id: None,
            classes: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn text_content(&self) -> String {
        let mut out = String::new();
        for child in &self.children {
            match child {
                Node::Text(t) => out.push_str(t),
                Node::Element(el) => out.push_str(&el.text_content()),
            }
        }
        out
    }

    pub fn has_class(&self, class_name: &str) -> bool {
        self.classes.iter().any(|c| c == class_name)
    }

    pub fn find_by_tag(&self, tag: &str) -> Option<&Element> {
        if self.tag == tag {
            return Some(self);
        }
        for child in &self.children {
            if let Node::Element(el) = child {
                if let Some(found) = el.find_by_tag(tag) {
                    return Some(found);
                }
            }
        }
        None
    }

    pub fn find_by_class(&self, class_name: &str) -> Option<&Element> {
        if self.has_class(class_name) {
            return Some(self);
        }
        for child in &self.children {
            if let Node::Element(el) = child {
                if let Some(found) = el.find_by_class(class_name) {
                    return Some(found);
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
}

pub struct HtmlDocument {
    pub root: Element,
    pub title: Option<String>,
}

impl HtmlDocument {
    pub fn parse(html: &str) -> Self {
        let nodes = parse_nodes(html);
        let mut root = Element::new("root");
        for n in nodes {
            root.children.push(n);
        }

        let title = root.find_by_tag("title").map(|el| el.text_content().trim().into());

        Self { root, title }
    }
}

fn parse_nodes(html: &str) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut rest = html.trim();

    while !rest.is_empty() {
        if rest.starts_with("<!--") {
            if let Some(end) = rest.find("-->") {
                rest = rest[end + 3..].trim();
                continue;
            } else {
                break;
            }
        }

        if rest.starts_with("<!") || rest.starts_with("<?") {
            if let Some(end) = rest.find('>') {
                rest = rest[end + 1..].trim();
                continue;
            } else {
                break;
            }
        }

        if rest.starts_with("</") {
            // Stray closing tag
            if let Some(end) = rest.find('>') {
                rest = rest[end + 1..].trim();
                continue;
            } else {
                break;
            }
        }

        if rest.starts_with('<') {
            if let Some(close_bracket) = rest.find('>') {
                let tag_content = &rest[1..close_bracket];
                let is_self_closing = tag_content.ends_with('/')
                    || tag_content.starts_with("meta")
                    || tag_content.starts_with("link")
                    || tag_content.starts_with("br")
                    || tag_content.starts_with("hr")
                    || tag_content.starts_with("img");

                let tag_raw = tag_content.trim_end_matches('/').trim();
                let mut el = parse_element_header(tag_raw);

                rest = rest[close_bracket + 1..].trim();

                if !is_self_closing {
                    let close_tag = alloc::format!("</{}>", el.tag);
                    if let Some(close_idx) = find_closing_tag(rest, &el.tag) {
                        let inner_html = &rest[..close_idx];
                        el.children = parse_nodes(inner_html);
                        rest = rest[close_idx + close_tag.len()..].trim();
                    }
                }

                nodes.push(Node::Element(el));
                continue;
            }

            // Unterminated tag (no closing '>'): keep the remainder as text and
            // stop, instead of spinning forever on the same input.
            let text = rest.trim();
            if !text.is_empty() {
                nodes.push(Node::Text(String::from(text)));
            }
            break;
        }

        // Text node
        let next_tag = rest.find('<').unwrap_or(rest.len());
        let text = rest[..next_tag].trim();
        if !text.is_empty() {
            nodes.push(Node::Text(String::from(text)));
        }
        rest = rest[next_tag..].trim();
    }

    nodes
}

fn find_closing_tag(s: &str, tag: &str) -> Option<usize> {
    let open_pattern = alloc::format!("<{}", tag);
    let close_pattern = alloc::format!("</{}>", tag);

    let mut depth = 0;
    let mut i = 0;

    while i < s.len() {
        // `i` advances byte by byte; never slice inside a multi-byte character.
        if !s.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if s[i..].starts_with(&close_pattern) {
            if depth == 0 {
                return Some(i);
            }
            depth -= 1;
            i += close_pattern.len();
        } else if s[i..].starts_with(&open_pattern) {
            let after = &s[i + open_pattern.len()..];
            if after.starts_with('>') || after.starts_with(' ') || after.starts_with('\t') || after.starts_with('\n') {
                depth += 1;
            }
            i += open_pattern.len();
        } else {
            i += 1;
        }
    }
    None
}

fn parse_element_header(tag_str: &str) -> Element {
    let tag_str = tag_str.trim();
    let name_end = tag_str.find(char::is_whitespace).unwrap_or(tag_str.len());
    let tag = &tag_str[..name_end];
    let mut el = Element::new(if tag.is_empty() { "div" } else { tag });

    // Attributes: name, name=value, name="value with spaces", name='value'.
    let mut rest = tag_str[name_end..].trim_start();
    while !rest.is_empty() {
        let name_end = rest
            .find(|c: char| c == '=' || c.is_whitespace())
            .unwrap_or(rest.len());
        let name = &rest[..name_end];
        rest = rest[name_end..].trim_start();

        let mut value = "";
        if let Some(after_eq) = rest.strip_prefix('=') {
            let after_eq = after_eq.trim_start();
            match after_eq.chars().next() {
                Some(q) if q == '"' || q == '\'' => {
                    let body = &after_eq[1..];
                    match body.find(q) {
                        Some(end) => {
                            value = &body[..end];
                            rest = body[end + 1..].trim_start();
                        }
                        None => {
                            value = body;
                            rest = "";
                        }
                    }
                }
                _ => {
                    let end = after_eq.find(char::is_whitespace).unwrap_or(after_eq.len());
                    value = &after_eq[..end];
                    rest = after_eq[end..].trim_start();
                }
            }
        }

        if name.eq_ignore_ascii_case("id") {
            el.id = Some(String::from(value));
        } else if name.eq_ignore_ascii_case("class") {
            for c in value.split_whitespace() {
                el.classes.push(String::from(c));
            }
        }
    }
    el
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_theme_html() {
        let html = r#"<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>htmlboot</title>
    <link rel="stylesheet" href="theme.css">
  </head>
  <body>
    <div class="card">
      <h1>htmlboot</h1>
      <ul class="entries">
        <li class="entry selected">Test Linux</li>
      </ul>
    </div>
  </body>
</html>"#;
        let doc = HtmlDocument::parse(html);
        assert_eq!(doc.title.as_deref(), Some("htmlboot"));

        let card = doc.root.find_by_class("card").expect("finds .card");
        assert_eq!(card.tag, "div");

        let h1 = card.find_by_tag("h1").expect("finds h1");
        assert_eq!(h1.text_content(), "htmlboot");

        let entries = card.find_by_class("entries").expect("finds .entries");
        assert_eq!(entries.tag, "ul");
    }

    #[test]
    fn multiple_classes_and_quote_styles() {
        let doc = HtmlDocument::parse(
            r#"<ul><li class="entry selected" id='first'>One</li><li class=plain>Two</li></ul>"#,
        );
        let ul = doc.root.find_by_tag("ul").expect("finds ul");
        let first = ul.find_by_class("selected").expect("finds .selected");
        assert_eq!(first.tag, "li");
        assert!(first.has_class("entry"));
        assert!(first.has_class("selected"));
        assert_eq!(first.id.as_deref(), Some("first"));
        let plain = ul.find_by_class("plain").expect("unquoted class");
        assert_eq!(plain.text_content(), "Two");
    }

    #[test]
    fn non_ascii_text_does_not_panic() {
        let doc = HtmlDocument::parse(
            "<html><head><title>htmlboot \u{2014} \u{437}\u{430}\u{433}\u{440}\u{443}\u{437}\u{447}\u{438}\u{43a}</title></head>\
             <body><div class=\"card\"><p>na\u{ef}ve \u{e9} \u{2713}</p></div></body></html>",
        );
        assert_eq!(
            doc.title.as_deref(),
            Some("htmlboot \u{2014} \u{437}\u{430}\u{433}\u{440}\u{443}\u{437}\u{447}\u{438}\u{43a}")
        );
        let p = doc.root.find_by_tag("p").expect("finds p");
        assert_eq!(p.text_content(), "na\u{ef}ve \u{e9} \u{2713}");
    }

    #[test]
    fn unterminated_tag_does_not_hang() {
        let doc = HtmlDocument::parse("<div>ok</div><span class=\"x");
        assert!(doc.root.find_by_tag("div").is_some());
        assert!(doc
            .root
            .children
            .iter()
            .any(|n| matches!(n, Node::Text(t) if t.starts_with("<span"))));
    }
}
