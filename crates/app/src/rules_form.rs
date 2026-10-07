//! The "After download" rule editor in Settings → General: the half-filled new rule, and rules
//! described in plain words.

use rdm_core::Category;
use rdm_core::rules::{Action, Match, Rule};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    Ext,
    Site,
    Category,
}

impl When {
    pub const ALL: [When; 3] = [When::Ext, When::Site, When::Category];
}

impl fmt::Display for When {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            When::Ext => "File type is",
            When::Site => "From site",
            When::Category => "Category is",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Do {
    Mp3,
    Smaller,
    Extract,
    Move,
}

impl Do {
    pub const ALL: [Do; 4] = [Do::Mp3, Do::Smaller, Do::Extract, Do::Move];
}

impl fmt::Display for Do {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Do::Mp3 => "Convert to MP3",
            Do::Smaller => "Make video smaller",
            Do::Extract => "Unpack archive",
            Do::Move => "Move to folder",
        })
    }
}

/// A category as the pick list shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CategoryChoice(pub Category);

impl CategoryChoice {
    pub const ALL: [CategoryChoice; 6] = [
        CategoryChoice(Category::Video),
        CategoryChoice(Category::Music),
        CategoryChoice(Category::Image),
        CategoryChoice(Category::Archive),
        CategoryChoice(Category::Document),
        CategoryChoice(Category::Program),
    ];
}

impl fmt::Display for CategoryChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.folder())
    }
}

/// The new rule being filled in.
#[derive(Clone, Debug, PartialEq)]
pub struct RuleForm {
    pub when: When,
    /// Extensions or the site, as typed.
    pub value: String,
    pub category: CategoryChoice,
    pub action: Do,
    pub folder: String,
    pub keep_original: bool,
}

impl Default for RuleForm {
    fn default() -> Self {
        RuleForm { when: When::Ext, value: String::new(), category: CategoryChoice(Category::Video), action: Do::Extract, folder: String::new(), keep_original: true }
    }
}

impl RuleForm {
    /// The finished rule, or what is still missing.
    pub fn build(&self, id: u32) -> Result<Rule, &'static str> {
        let value = self.value.trim();
        let when = match self.when {
            When::Ext if value.is_empty() => return Err("Type a file type, like zip"),
            When::Site if value.is_empty() => return Err("Type a site, like youtube.com"),
            When::Ext => Match::Ext(value.to_string()),
            When::Site => Match::Site(site_of(value)),
            When::Category => Match::Category(self.category.0),
        };
        let action = match self.action {
            Do::Mp3 => Action::ToMp3,
            Do::Smaller => Action::SmallerMp4 { crf: 28 },
            Do::Extract => Action::Extract,
            Do::Move if self.folder.trim().is_empty() => return Err("Type the folder to move to"),
            Do::Move => Action::MoveTo(self.folder.trim().into()),
        };
        Ok(Rule { id, enabled: true, when, action, keep_original: self.keep_original })
    }
}

/// "https://www.youtube.com/watch…" or "www.youtube.com" → "youtube.com".
fn site_of(typed: &str) -> String {
    let host = typed.split("://").last().unwrap_or(typed).split(['/', '?', '#']).next().unwrap_or("");
    host.trim_start_matches("www.").to_ascii_lowercase()
}

/// A rule in plain words.
pub fn describe(rule: &Rule) -> String {
    let when = match &rule.when {
        Match::Ext(list) => {
            let exts: Vec<String> = list.split(',').map(|e| format!(".{}", e.trim().trim_start_matches('.').to_ascii_lowercase())).filter(|e| e.len() > 1).collect();
            format!("{} files", exts.join(", "))
        }
        Match::Site(site) => format!("From {site}"),
        Match::Category(c) => c.folder().to_string(),
    };
    let action = match &rule.action {
        Action::ToMp3 => "convert to MP3".to_string(),
        Action::SmallerMp4 { .. } => "make a smaller MP4".to_string(),
        Action::Extract => "unpack".to_string(),
        Action::MoveTo(dir) => format!("move to {}", dir.display()),
    };
    let keep = if rule.keep_original { " (keep the original)" } else { "" };
    format!("{when} → {action}{keep}")
}

/// The id for a new rule: one past the highest in use.
pub fn next_id(rules: &[Rule]) -> u32 {
    rules.iter().map(|r| r.id).max().map_or(1, |m| m + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filled_form_becomes_a_rule() {
        let form = RuleForm { when: When::Site, value: "https://www.YouTube.com/watch?v=1".into(), action: Do::Mp3, keep_original: false, ..RuleForm::default() };
        let rule = form.build(3).unwrap();
        assert_eq!(rule.id, 3);
        assert_eq!(rule.when, Match::Site("youtube.com".into()), "a pasted link becomes its site");
        assert_eq!(rule.action, Action::ToMp3);
        assert!(!rule.keep_original && rule.enabled);
    }

    #[test]
    fn missing_parts_are_named() {
        assert!(RuleForm { value: "  ".into(), ..RuleForm::default() }.build(1).unwrap_err().contains("file type"));
        let mv = RuleForm { value: "exe".into(), action: Do::Move, ..RuleForm::default() };
        assert!(mv.build(1).unwrap_err().contains("folder"));
        let cat = RuleForm { when: When::Category, value: String::new(), category: CategoryChoice(Category::Program), action: Do::Move, folder: r"D:\Apps".into(), ..RuleForm::default() };
        assert_eq!(cat.build(1).unwrap().when, Match::Category(Category::Program), "a category needs no typing");
    }

    #[test]
    fn rules_read_as_plain_words() {
        let rule = RuleForm { value: "ZIP, .7z".into(), ..RuleForm::default() }.build(1).unwrap();
        assert_eq!(describe(&rule), ".zip, .7z files → unpack (keep the original)");
        let rule = RuleForm { when: When::Category, category: CategoryChoice(Category::Video), action: Do::Smaller, keep_original: false, ..RuleForm::default() }.build(1).unwrap();
        assert_eq!(describe(&rule), "Videos → make a smaller MP4");
        assert_eq!(next_id(&[]), 1);
        assert_eq!(next_id(&[rule.clone(), Rule { id: 7, ..rule }]), 8);
    }
}
