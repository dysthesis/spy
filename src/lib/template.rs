use minijinja::{AutoEscape, Environment, UndefinedBehavior, Value};
use once_cell::sync::Lazy;
use thiserror::Error;

use crate::entry::{Entry, EntryTemplateContext};

pub static ENVIRONMENT: Lazy<Environment> = Lazy::new(|| {
    let mut e = Environment::new();
    e.set_undefined_behavior(UndefinedBehavior::Strict);
    e.set_auto_escape_callback(|_| AutoEscape::None);
    e
});

#[derive(Error, Debug)]
pub enum Error {
    #[error(
        r#"Failed to initialise template string.
            Template:   {template}
            Error:      {error}"#
    )]
    TemplateInitialisationError {
        template: String,
        error: Box<minijinja::Error>,
    },
    #[error(
        r#"Failed to render template string.
            Template:   {template}
            Entry:     {entry}
            Error:      {error}"#
    )]
    RenderFailure {
        template: String,
        entry: Box<Entry>,
        error: Box<minijinja::Error>,
    },
}

/// A template string
pub struct Template(String);
impl Template {
    pub fn new(string: String) -> Self {
        Self(string)
    }
    /// Substitute the keys in a template string with the given values
    pub fn render(&self, entry: &Entry) -> Result<String, Error> {
        let template = ENVIRONMENT.template_from_str(&self.0).map_err(|e| {
            Error::TemplateInitialisationError {
                template: self.0.clone(),
                error: Box::new(e),
            }
        })?;
        let data = Value::from_serialize(EntryTemplateContext::new(entry));
        template.render(data).map_err(|e| Error::RenderFailure {
            template: self.0.clone(),
            entry: Box::new(entry.clone()),
            error: Box::new(e),
        })
    }
}

/// Build the MiniJinja value map used to render an entry.
pub fn context_value(entry: &Entry) -> Value {
    Value::from_serialize(EntryTemplateContext::new(entry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    use crate::entry::{EntryTemplateContext, entry_strategy};

    const KNOWN_FIELDS: &[&str] = &[
        "title",
        "site",
        "author",
        "authors",
        "url",
        "id",
        "description",
        "thumbnail",
        "full_text",
        "entry",
    ];

    fn unknown_field_strategy() -> proptest::strategy::BoxedStrategy<String> {
        prop::string::string_regex("[a-z_]{5,16}")
            .unwrap()
            .prop_filter("field must be undefined", |s| {
                !KNOWN_FIELDS.contains(&s.as_str())
            })
            .boxed()
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000_000))]
        #[test]
        fn rendering_unknown_field_is_error(entry in entry_strategy(), field in unknown_field_strategy()) {
            let template = Template::new(format!("{{{{ {field} }}}}"));
            let result = template.render(&entry);
            let failed = matches!(result, Err(Error::RenderFailure { .. }));
            prop_assert!(failed);
        }
    }

    proptest! {
        #[test]
        fn rendering_known_title_matches_context(entry in entry_strategy()) {
            let output = Template::new("{{ title }}".to_string()).render(&entry).unwrap();
            let context = EntryTemplateContext::new(&entry);
            let json = serde_json::to_value(context).unwrap();
            let expected = json.get("title").and_then(|v| v.as_str()).unwrap();
            prop_assert_eq!(output, expected);
        }
    }
}
