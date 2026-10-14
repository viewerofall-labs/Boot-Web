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
                    // CSS custom properties (--foo: bar) contain a colon in the
                    // name, so we split on the *first* colon only.
                    if let Some(colon) = item.find(':') {
                        let k = item[..colon].trim();
                        let v = item[colon + 1..].trim();
                        declarations.push(Declaration {
                            // Preserve case for custom properties (--Foo stays --Foo),
                            // lower-case everything else.
                            property: if k.starts_with("--") {
                                String::from(k)
                            } else {
                                String::from(k.to_ascii_lowercase())
                            },
                            value: String::from(v),
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

    /// Look up a property in a selector's rules (last rule wins).
    pub fn get_property(&self, selector: &str, property: &str) -> Option<&str> {
        // Custom properties are case-sensitive; regular ones are lower-cased.
        let property_lc;
        let prop_key: &str = if property.starts_with("--") {
            property
        } else {
            property_lc = property.to_ascii_lowercase();
            &property_lc
        };

        for rule in self.rules.iter().rev() {
            if rule.selector == selector {
                for decl in &rule.declarations {
                    if decl.property == prop_key {
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

    /// Read a CSS custom property (`--name`) from `:root`.
    pub fn get_var(&self, name: &str) -> Option<&str> {
        self.get_property(":root", name)
    }

    /// Read a CSS custom property as a signed integer.
    pub fn get_var_int(&self, name: &str) -> Option<i32> {
        parse_css_int(self.get_var(name)?)
    }

    /// Read a CSS custom property as a percentage in the range 0–100
    /// (fixed-point ×1, so "50%" → 50, "0.5" → 50, "25" → 25).
    pub fn get_var_pct(&self, name: &str) -> Option<i32> {
        parse_css_pct(self.get_var(name)?)
    }
}

/// Parse a CSS integer value: "44", "44px" → Some(44).
pub fn parse_css_int(s: &str) -> Option<i32> {
    let s = s.trim().trim_end_matches("px").trim();
    s.parse::<i32>().ok()
}

/// Parse a CSS percentage or decimal fraction into an integer percentage 0–100.
///
/// | Input  | Result |
/// |--------|--------|
/// | "50%"  | 50     |
/// | "0.50" | 50     |
/// | "1.0"  | 100    |
/// | "25"   | 25     |
pub fn parse_css_pct(s: &str) -> Option<i32> {
    let s = s.trim();
    if let Some(s) = s.strip_suffix('%') {
        return s.trim().parse::<i32>().ok();
    }
    if let Some((int_part, frac_part)) = s.split_once('.') {
        let int_val: i32 = int_part.trim().parse().ok().unwrap_or(0);
        // Take up to 2 decimal digits
        let frac_str = if frac_part.len() >= 2 { &frac_part[..2] } else { frac_part };
        let frac_val: i32 = frac_str.parse().ok().unwrap_or(0);
        let frac_scaled = if frac_part.len() >= 2 { frac_val } else { frac_val * 10 };
        return Some(int_val * 100 + frac_scaled);
    }
    s.parse::<i32>().ok()
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

    #[test]
    fn test_css_custom_properties() {
        let css = r#"
            :root {
                --item-height: 44px;
                --title-scale: 2;
                --card-x-pct: 50%;
                --card-w-pct: 0.80;
            }
        "#;
        let sheet = Stylesheet::parse(css);
        assert_eq!(sheet.get_var_int("--item-height"), Some(44));
        assert_eq!(sheet.get_var_int("--title-scale"), Some(2));
        assert_eq!(sheet.get_var_pct("--card-x-pct"), Some(50));
        assert_eq!(sheet.get_var_pct("--card-w-pct"), Some(80));
    }

    #[test]
    fn test_parse_css_pct() {
        assert_eq!(parse_css_pct("50%"), Some(50));
        assert_eq!(parse_css_pct("0.50"), Some(50));
        assert_eq!(parse_css_pct("0.5"), Some(50));
        assert_eq!(parse_css_pct("1.0"), Some(100));
        assert_eq!(parse_css_pct("25"), Some(25));
    }
}
