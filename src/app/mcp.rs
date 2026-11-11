//! The MCP servers, through which AI agents read oxploy's documentation.
//!
//! `/mcp` is public: any agent lists, reads and searches the documentation there.
//! `/admin/mcp` is the owner's, which oxploy reaches with the token it gives the site,
//! and also reports the site's visits. They answer the same way, with the tools each
//! one's [`Access`] reaches.
//!
//! Both speak the Model Context Protocol over Streamable HTTP in its simplest form: each
//! POST carries one JSON-RPC message and gets plain JSON back, with no session and no
//! event stream, which is all a server with only tools needs. Each tool is a file of its
//! own in `src/app/mcp/`, with a line in `definitions` and in `call_tool`.

mod common;
mod list_docs;
mod read_doc;
mod search_docs;
mod site_visits;

use serde_json::{Value, json};
use topcoat::{
    Result,
    context::Cx,
    router::{
        StatusCode,
        content::Json,
        request::Bytes,
        response::{IntoResponse, Response},
        route,
    },
};

use self::common::ToolError;

/// Who is calling, which decides the tools they reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Anyone, at `/mcp`: the documentation's tools.
    Anyone,
    /// oxploy, with the site's token, at `/admin/mcp`: those, and the site's visits.
    Owner,
}

/// The protocol versions the servers speak, newest first. The few methods they answer
/// read the same in all of them.
const PROTOCOL_VERSIONS: [&str; 3] = ["2025-11-25", "2025-06-18", "2025-03-26"];

/// JSON-RPC's error codes.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

/// What the servers tell an agent about themselves when it connects.
const INSTRUCTIONS: &str = "oxploy is a self-hosted deploy panel for Rust: it builds \
    apps from GitHub with cargo, runs them as processes on your own server and serves \
    them through Caddy with HTTPS, without Docker. This server reads its documentation, \
    the pages at https://oxploy.com/docs: search_docs finds the passages about a \
    question, read_doc gives a whole page, and list_docs every page with its headings. \
    A page's slug is the end of its address, such as domains in \
    https://oxploy.com/docs/domains. Answer from what the pages say, and link to them.";

/// What the owner's server adds to [`INSTRUCTIONS`].
const OWNER_INSTRUCTIONS: &str = " site_visits also reports the visits to the site, as \
    its admin at https://oxploy.com/admin shows them.";

/// Receives one MCP message from anyone, and answers it with the documentation's tools.
#[route(POST)]
pub async fn receive(cx: &Cx, body: Bytes) -> Result<Response> {
    respond(cx, &body, Access::Anyone).await
}

/// Answers the MCP message in `body` with the tools `access` reaches.
pub async fn respond(cx: &Cx, body: &[u8], access: Access) -> Result<Response> {
    let answer = match serde_json::from_slice(body) {
        Ok(message) => answer(cx, &message, access).await,
        Err(_) => Some(failure(Value::Null, PARSE_ERROR, "The body is not JSON.")),
    };
    match answer {
        Some(answer) => Json(answer).into_response(cx),
        // A notification gets no answer, only the acknowledgement the protocol asks for.
        None => StatusCode::ACCEPTED.into_response(cx),
    }
}

/// A JSON-RPC error: its code and what went wrong.
#[derive(Debug)]
struct Failure {
    code: i64,
    message: String,
}

impl Failure {
    fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// The answer to one JSON-RPC message, or `None` for a notification, such as
/// `notifications/initialized`, which gets none.
async fn answer(cx: &Cx, message: &Value, access: Access) -> Option<Value> {
    let id = message.get("id").cloned();
    let Some(method) = message.get("method").and_then(Value::as_str) else {
        let id = id.unwrap_or(Value::Null);
        return Some(failure(id, INVALID_REQUEST, "Expected one JSON-RPC request."));
    };
    let id = id?;
    let params = message.get("params").unwrap_or(&Value::Null);
    let outcome = match method {
        "initialize" => Ok(initialize(params, access)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": definitions(access) })),
        "tools/call" => call_tool(cx, params, access).await,
        _ => Err(Failure::new(METHOD_NOT_FOUND, format!("Unknown method: {method}."))),
    };
    Some(match outcome {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => failure(id, error.code, &error.message),
    })
}

/// The answer to `initialize`: the protocol version, the client's when the server
/// speaks it or else the newest, and what the server offers.
fn initialize(params: &Value, access: Access) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    let version = requested
        .filter(|version| PROTOCOL_VERSIONS.contains(version))
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    let instructions = match access {
        Access::Anyone => INSTRUCTIONS.to_owned(),
        Access::Owner => format!("{INSTRUCTIONS}{OWNER_INSTRUCTIONS}"),
    };
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": {
            "name": "oxploy-docs",
            "title": "oxploy docs",
            "version": env!("CARGO_PKG_VERSION"),
            "websiteUrl": "https://oxploy.com/docs",
        },
        "instructions": instructions,
    })
}

/// The name, description and arguments of every tool `access` reaches, for
/// `tools/list`.
fn definitions(access: Access) -> Vec<Value> {
    let mut tools = vec![
        search_docs::definition(),
        read_doc::definition(),
        list_docs::definition(),
    ];
    if access == Access::Owner {
        tools.push(site_visits::definition());
    }
    tools
}

/// Runs the tool `params` names. A call the tool refuses comes back as the tool's own
/// error, which the agent reads and can correct; an unknown tool, or one `access` does
/// not reach, or a database failure is an error of the protocol.
async fn call_tool(cx: &Cx, params: &Value, access: Access) -> Result<Value, Failure> {
    let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
    let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let outcome = match name {
        search_docs::NAME => search_docs::call(cx, arguments),
        read_doc::NAME => read_doc::call(cx, arguments),
        list_docs::NAME => list_docs::call(arguments),
        site_visits::NAME if access == Access::Owner => site_visits::call(cx, arguments).await,
        _ => return Err(Failure::new(INVALID_PARAMS, format!("Unknown tool: {name}."))),
    };
    match outcome {
        Ok(data) => Ok(tool_result(data.to_string(), false)),
        Err(ToolError::Refused(why)) => Ok(tool_result(why, true)),
        Err(ToolError::Failed(error)) => {
            eprintln!("oxploy-website: the MCP tool {name} failed: {error}");
            Err(Failure::new(INTERNAL_ERROR, "The tool failed. Try again later."))
        }
    }
}

/// A tool's result: its text, and whether it is an error.
fn tool_result(text: String, is_error: bool) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": is_error })
}

/// A JSON-RPC error response.
fn failure(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use topcoat::context::CxTestBuilder;

    use super::*;

    async fn send(access: Access, message: Value) -> Option<Value> {
        answer(&CxTestBuilder::new().build(), &message, access).await
    }

    async fn call(access: Access, name: &str, arguments: Value) -> Value {
        let message = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments },
        });
        send(access, message).await.unwrap()
    }

    /// The JSON a tool's call gave.
    fn data(answer: &Value) -> Value {
        assert_eq!(answer["result"]["isError"], false, "{answer}");
        serde_json::from_str(answer["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    async fn names(access: Access) -> Vec<String> {
        let answer = send(access, json!({ "jsonrpc": "2.0", "id": "a", "method": "tools/list" }))
            .await
            .unwrap();
        assert_eq!(answer["id"], "a");
        let tools = answer["result"]["tools"].as_array().unwrap();
        for tool in tools {
            assert_eq!(tool["inputSchema"]["type"], "object", "{}", tool["name"]);
            assert_eq!(tool["annotations"]["readOnlyHint"], true, "{}", tool["name"]);
        }
        tools.iter().map(|tool| tool["name"].as_str().unwrap().to_owned()).collect()
    }

    #[tokio::test]
    async fn speaks_the_clients_version_or_its_newest() {
        let initialize = |version: &str| {
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": { "protocolVersion": version },
            })
        };
        let answer = send(Access::Anyone, initialize("2025-06-18")).await.unwrap();
        assert_eq!(answer["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(answer["id"], 1);
        let answer = send(Access::Anyone, initialize("2030-01-01")).await.unwrap();
        assert_eq!(answer["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
    }

    #[tokio::test]
    async fn tells_only_the_owner_about_the_visits() {
        let message = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} });
        let anyone = send(Access::Anyone, message.clone()).await.unwrap();
        let owner = send(Access::Owner, message).await.unwrap();
        assert!(!anyone["result"]["instructions"].as_str().unwrap().contains("site_visits"));
        assert!(owner["result"]["instructions"].as_str().unwrap().contains("site_visits"));
    }

    #[tokio::test]
    async fn answers_no_notification() {
        let message = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert_eq!(send(Access::Anyone, message).await, None);
    }

    #[tokio::test]
    async fn gives_the_visits_only_to_the_owner() {
        assert_eq!(names(Access::Anyone).await, ["search_docs", "read_doc", "list_docs"]);
        assert_eq!(names(Access::Owner).await, ["search_docs", "read_doc", "list_docs", "site_visits"]);
        let answer = call(Access::Anyone, site_visits::NAME, json!({})).await;
        assert_eq!(answer["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn finds_reads_and_lists_the_pages() {
        let found = data(&call(Access::Anyone, search_docs::NAME, json!({ "query": "custom domain https" })).await);
        let first = &found["results"][0];
        assert_eq!(first["page"], "domains", "{found}");
        assert!(first["url"].as_str().unwrap().starts_with("https://oxploy.com/docs/domains"));

        let page = data(&call(Access::Anyone, read_doc::NAME, json!({ "slug": "storage" })).await);
        assert_eq!(page["title"], "Storage");
        assert_eq!(page["url"], "https://oxploy.com/docs/storage");
        assert_eq!(page["previous"], "domains");
        assert!(!page["markdown"].as_str().unwrap().contains("](/"));

        let listed = data(&call(Access::Anyone, list_docs::NAME, json!({})).await);
        let sections = listed["sections"].as_array().unwrap();
        assert_eq!(sections[0]["pages"][0]["slug"], "introduction");
        let headings = &sections[0]["pages"][1]["headings"];
        assert!(headings.as_array().unwrap().iter().any(|heading| {
            heading["url"] == "https://oxploy.com/docs/installation#run-it-as-a-service"
        }), "{headings}");
    }

    #[tokio::test]
    async fn a_refused_call_is_the_tools_own_error() {
        for (name, arguments, why) in [
            (read_doc::NAME, json!({ "slug": "kubernetes" }), "introduction"),
            (read_doc::NAME, json!({}), "missing field"),
            (search_docs::NAME, json!({ "query": "caddy", "limt": 3 }), "unknown field"),
            (search_docs::NAME, json!({ "query": "caddy", "limit": 0 }), "limit"),
            (search_docs::NAME, json!({ "query": " ? " }), "word"),
        ] {
            let answer = call(Access::Anyone, name, arguments).await;
            assert_eq!(answer["result"]["isError"], true, "{answer}");
            let text = answer["result"]["content"][0]["text"].as_str().unwrap();
            assert!(text.contains(why), "{text}");
        }
        let answer = call(Access::Owner, site_visits::NAME, json!({ "days": 365 })).await;
        assert_eq!(answer["result"]["isError"], true, "{answer}");
    }

    #[tokio::test]
    async fn refuses_what_it_does_not_know() {
        let unknown_method = send(Access::Anyone, json!({ "jsonrpc": "2.0", "id": 1, "method": "resources/list" }))
            .await
            .unwrap();
        assert_eq!(unknown_method["error"]["code"], METHOD_NOT_FOUND);

        let unknown_tool = call(Access::Owner, "delete_visits", json!({})).await;
        assert_eq!(unknown_tool["error"]["code"], INVALID_PARAMS);

        let batch = send(Access::Anyone, json!([{ "jsonrpc": "2.0", "id": 3, "method": "ping" }]))
            .await
            .unwrap();
        assert_eq!(batch["error"]["code"], INVALID_REQUEST);
    }
}
