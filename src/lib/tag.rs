use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

pub static TAG_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_-]{1,30}$").expect("Tag regex is invalid!"));

#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Tag(String);

impl TryFrom<&&str> for Tag {
    type Error = ();
    fn try_from(tag: &&str) -> Result<Self, Self::Error> {
        let trimmed_tag = tag.trim();
        if trimmed_tag.is_empty() {
            return Err(());
        }
        if !TAG_RE.is_match(trimmed_tag) {
            return Err(());
        }

        Ok(Self(trimmed_tag.to_lowercase().to_string()))
    }
}

impl TryFrom<&str> for Tag {
    type Error = ();

    fn try_from(tag: &str) -> Result<Self, Self::Error> {
        Self::try_from(&tag)
    }
}
