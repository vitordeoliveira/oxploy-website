//! The `list_docs` tool: every page of the documentation, as its sidebar orders them.

use serde::Deserialize;
use serde_json::{Value, json};

use super::common::{ToolError, headings, parse, url};
use crate::features::docs::SECTIONS;

/// The name agents call the tool by.
pub const NAME: &str = "list_docs";

/// What the tool does, for the agent deciding whether to call it.
const DESCRIPTION: &str = "Lists every page of oxploy's documentation, in the order \
    its sidebar shows them, by section: each page's slug, title, a sentence about it, \
    its address, and its headings with their addresses. read_doc reads a page by its \
    slug.";

/// The tool's name, description and arguments, for `tools/list`.
pub fn definition() -> Value {
    json!({
        "name": NAME,
        "title": "List the pages of the documentation",
        "description": DESCRIPTION,
        "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
        "annotations": { "readOnlyHint": true, "openWorldHint": false },
    })
}

/// The tool takes no arguments.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {}

/// Lists the pages.
pub fn call(arguments: Value) -> Result<Value, ToolError> {
    let Arguments {} = parse(arguments)?;
    let sections: Vec<Value> = SECTIONS
        .iter()
        .map(|section| {
            let pages: Vec<Value> = section
                .pages
                .iter()
                .map(|page| {
                    json!({
                        "slug": page.slug,
                        "title": page.title,
                        "summary": page.summary,
                        "url": url(page, None),
                        "headings": headings(page),
                    })
                })
                .collect();
            json!({ "title": section.title, "pages": pages })
        })
        .collect();
    Ok(json!({ "sections": sections }))
}
