extern crate alloc;

use alloc::string::String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BootEntry {
    pub id: String,
    pub title: String,
    pub version: Option<String>,
    pub linux: String,
    pub initrd: Option<String>,
    pub options: Option<String>,
}

impl BootEntry {
    pub fn parse(id: &str, content: &str) -> Option<Self> {
        let mut title = None;
        let mut version = None;
        let mut linux = None;
        let mut initrd = None;
        let mut options = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = line.split_once(char::is_whitespace) {
                let key = key.trim();
                let val = val.trim();
                match key {
                    "title" => title = Some(String::from(val)),
                    "version" => version = Some(String::from(val)),
                    "linux" => linux = Some(String::from(val)),
                    "initrd" => initrd = Some(String::from(val)),
                    "options" => options = Some(String::from(val)),
                    _ => {}
                }
            }
        }

        let linux = linux?;
        let title = title.unwrap_or_else(|| String::from(id));

        Some(BootEntry {
            id: String::from(id),
            title,
            version,
            linux,
            initrd,
            options,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bls() {
        let sample = "title Test Linux\nlinux /vmlinuz\ninitrd /initramfs.gz\noptions console=tty0 console=ttyS0\n";
        let entry = BootEntry::parse("test.conf", sample).expect("valid BLS entry");
        assert_eq!(entry.id, "test.conf");
        assert_eq!(entry.title, "Test Linux");
        assert_eq!(entry.linux, "/vmlinuz");
        assert_eq!(entry.initrd.as_deref(), Some("/initramfs.gz"));
        assert_eq!(entry.options.as_deref(), Some("console=tty0 console=ttyS0"));
    }
}
