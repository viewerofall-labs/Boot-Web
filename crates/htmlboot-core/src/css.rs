extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::{parse_hex_color, Color};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub property: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub selector: String,
    pub declarations: Vec<Declaration>,
}

#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

impl Stylesheet {
    pub fn parse(css: &str) -> Self {
        let mut rules = Vec::new();
        let mut rest = css.trim();

        while let Some(open) = rest.find('{') {
            let selector = rest[..open].trim();
            let after = &rest[open + 1..];
            if let Some(close) = after.find('}') {
                let decls_str = &after[..close];
                let mut declarations = Vec::new();

                for item in decls_str.split(';') {
                    let item = item.trim();
                    if item.is_empty() || item.starts_with("/*") {
                        continue;
                    }
                    if let Some((k, v)) = item.split_once(':') {
                        declarations.push(Declaration {
                            property: String::from(k.trim().to_ascii_lowercase()),
                            value: String::from(v.trim()),
                        });
                    }
                }

                if !selector.is_empty() {
                    rules.push(Rule {
                        selector: String::from(selector),
                        declarations,
                    });
                }
                rest = after[close + 1..].trim();
            } else {
                break;
            }
        }

        Self { rules }
    }

    pub fn get_property(&self, selector: &str, property: &str) -> Option<&str> {
        let property = property.to_ascii_lowercase();
        // Look in reverse order so later rules override earlier ones
        for rule in self.rules.iter().rev() {
            if rule.selector == selector {
                for decl in &rule.declarations {
                    if decl.property == property {
                        return Some(&decl.value);
                    }
                }
            }
        }
        None
    }

    pub fn get_color(&self, selector: &str, property: &str) -> Option<Color> {
        self.get_property(selector, property).and_then(parse_hex_color)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComputedStyle {
    pub bg_color: Option<Color>,
    pub fg_color: Option<Color>,
    pub border_color: Option<Color>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stylesheet() {
        let css = r#"
            body { background-color: #1e1e2e; color: #cdd6f4; }
            .card { background-color: #313244; border-color: #45475a; }
            .entry { background-color: #181825; color: #a6adc8; }
            .entry.selected { background-color: #89b4fa; color: #11111b; }
        "#;
        let sheet = Stylesheet::parse(css);
        assert_eq!(
            sheet.get_color("body", "background-color"),
            Some(Color::rgb(0x1e, 0x1e, 0x2e))
        );
        assert_eq!(
            sheet.get_color(".card", "background-color"),
            Some(Color::rgb(0x31, 0x32, 0x44))
        );
        assert_eq!(
            sheet.get_color(".entry.selected", "background-color"),
            Some(Color::rgb(0x89, 0xb4, 0xfa))
        );
    }
}
