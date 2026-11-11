//! The `read_doc` tool: a whole page of the documentation.

use serde::Deserialize;
use serde_json::{Value, json};
use topcoat::{
    asset::AssetConfig,
    context::{Cx, try_app_context},
};

use super::common::{ToolError, parse, refuse, url};
use crate::features::docs::{self, Page};

/// The name agents call the tool by.
pub const NAME: &str = "read_doc";

/// What the tool does, for the agent deciding whether to call it.
const DESCRIPTION: &str = "Reads a whole page of oxploy's documentation, in Markdown, \
    with its title, a sentence about it, its address, and the pages before and after \
    it. Its links to other pages are absolute, such as \
    https://oxploy.com/docs/domains, whose slug is domains.";

/// The tool's name, description and arguments, for `tools/list`.
pub fn definition() -> Value {
    let slugs: Vec<&str> = docs::pages().map(|page| page.slug).collect();
    json!({
        "name": NAME,
        "title": "Read a page of the documentation",
        "description": DESCRIPTION,
        "inputSchema": {
            "type": "object",
            "properties": {
                "slug": {
                    "type": "string",
                    "enum": slugs,
                    "description": "The page, by the end of its address.",
                },
            },
            "required": ["slug"],
            "additionalProperties": false,
        },
        "annotations": { "readOnlyHint": true, "openWorldHint": false },
    })
}

/// The tool's arguments, as the agent sends them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    slug: String,
}

/// Reads the page the arguments name.
pub fn call(cx: &Cx, arguments: Value) -> Result<Value, ToolError> {
    let Arguments { slug } = parse(arguments)?;
    let page = docs::find(&slug).ok_or_else(|| {
        let slugs: Vec<&str> = docs::pages().map(|page| page.slug).collect();
        refuse(format!("There is no page {slug}. Use one of: {}.", slugs.join(", ")))
    })?;
    Ok(read(page, try_app_context::<AssetConfig>(cx)))
}

/// The page as the tool gives it, with its screenshots where `assets` serves them.
fn read(page: &Page, assets: Option<&AssetConfig>) -> Value {
    let (previous, next) = page.neighbors();
    json!({
        "slug": page.slug,
        "title": page.title,
        "summary": page.summary,
        "section": page.section().title,
        "url": url(page, None),
        "markdown": page.absolute_markdown(assets),
        "previous": previous.map(|page| page.slug),
        "next": next.map(|page| page.slug),
    })
}
