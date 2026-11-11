//! What the tools share: how a call fails, how its arguments are read, and how a page
//! is named.

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::features::docs::{Heading, Page};
use crate::features::seo;

/// Why a tool call did not go through.
#[derive(Debug)]
pub enum ToolError {
    /// The call asks for something the tool cannot do, such as a page that does not
    /// exist. The agent reads why and can correct the call.
    Refused(String),
    /// The database failed.
    Failed(topcoat::Error),
}

impl From<topcoat::Error> for ToolError {
    fn from(error: topcoat::Error) -> Self {
        ToolError::Failed(error)
    }
}

/// A refusal that says why.
pub fn refuse(why: impl Into<String>) -> ToolError {
    ToolError::Refused(why.into())
}

/// Reads a tool's arguments. They refuse unknown fields, so a misspelled one is never
/// silently ignored.
pub fn parse<T: DeserializeOwned>(arguments: Value) -> Result<T, ToolError> {
    serde_json::from_value(arguments).map_err(|error| refuse(format!("Invalid arguments: {error}.")))
}

/// The address of `page` on the site, at `heading` when there is one, such as
/// `https://oxploy.com/docs/installation#run-it-as-a-service`.
pub fn url(page: &Page, heading: Option<&Heading>) -> String {
    match heading {
        Some(heading) => seo::url(&format!("/docs/{}#{}", page.slug, heading.id)),
        None => seo::url(&format!("/docs/{}", page.slug)),
    }
}

/// `page`'s headings, each with its address.
pub fn headings(page: &Page) -> Vec<Value> {
    page.rendered()
        .headings
        .iter()
        .map(|heading| {
            json!({
                "text": heading.text,
                "url": url(page, Some(heading)),
                "nested": heading.nested,
            })
        })
        .collect()
}
