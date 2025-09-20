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
