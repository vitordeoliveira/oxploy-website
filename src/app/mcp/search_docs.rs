//! The `search_docs` tool: the passages of the documentation about a question.

use serde::Deserialize;
use serde_json::{Value, json};
use topcoat::{
    asset::AssetConfig,
    context::{Cx, try_app_context},
};

use super::common::{ToolError, parse, refuse, url};
use crate::features::docs::search::{Hit, has_words, search};

/// The name agents call the tool by.
pub const NAME: &str = "search_docs";

/// The most passages a search gives.
const MAX_LIMIT: usize = 10;

/// The passages given when the agent asks for no number.
const DEFAULT_LIMIT: usize = 5;

/// The longest query the tool takes, in characters.
const MAX_QUERY: usize = 200;

/// What the tool does, for the agent deciding whether to call it.
const DESCRIPTION: &str = "Searches oxploy's documentation, and gives the passages \
    that best match the query, best first: each is a page's part under one heading, in \
    Markdown, with the address it is at. Search with the words the pages would use, \
    such as \"custom domain https\", \"OXPLOY_STORAGE_DIR\" or \"back up a SQLite \
    database\". A word finds the ones it starts, so domain finds domains. Read the \
    whole page with read_doc when a passage is not enough.";

/// The tool's name, description and arguments, for `tools/list`.
pub fn definition() -> Value {
    json!({
        "name": NAME,
        "title": "Search the documentation",
        "description": DESCRIPTION,
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": MAX_QUERY,
                    "description": "The words to search for.",
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_LIMIT,
                    "default": DEFAULT_LIMIT,
                    "description": "How many passages to give at most.",
                },
            },
            "required": ["query"],
            "additionalProperties": false,
        },
        "annotations": { "readOnlyHint": true, "openWorldHint": false },
    })
}

/// The tool's arguments, as the agent sends them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    query: String,
    limit: Option<usize>,
}

/// Searches the documentation for the query the arguments give.
pub fn call(cx: &Cx, arguments: Value) -> Result<Value, ToolError> {
    let Arguments { query, limit } = parse(arguments)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(refuse(format!("limit must be from 1 to {MAX_LIMIT}, not {limit}.")));
    }
    if query.chars().count() > MAX_QUERY {
        return Err(refuse(format!("The query has more than {MAX_QUERY} characters.")));
    }
    if !has_words(&query) {
        return Err(refuse("The query has no word of two letters or more to search for."));
    }
    let assets = try_app_context::<AssetConfig>(cx);
    let results: Vec<Value> = search(&query, limit, assets).iter().map(result).collect();
    Ok(json!({ "query": query, "results": results }))
}

/// A passage found, as the tool gives it.
fn result(hit: &Hit) -> Value {
    json!({
        "page": hit.page.slug,
        "page_title": hit.page.title,
        "heading": hit.heading.map(|heading| heading.text.as_str()),
        "url": url(hit.page, hit.heading),
        "markdown": hit.markdown,
    })
}
