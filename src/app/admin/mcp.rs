//! The owner's MCP server, at `/admin/mcp`: the documentation's tools and the site's
//! visits, for the token oxploy gives the site. In oxploy, the app's MCP tab turns it on
//! at this path, and Caddy then hides the path from the site's domains, so only oxploy
//! reaches it.

use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    router::{
        HeaderValue, StatusCode,
        header::WWW_AUTHENTICATE,
        request::Bytes,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::app::mcp::{Access, respond};
use crate::features::mcp::is_authorized;

/// How long a request without the right token waits before the answer, which slows
/// guessing.
const WRONG_TOKEN_WAIT: Duration = Duration::from_secs(1);

/// Receives one MCP message from the owner, and answers it with every tool.
#[route(POST)]
pub async fn receive(cx: &Cx, body: Bytes) -> Result<Response> {
    if !is_authorized(cx) {
        tokio::time::sleep(WRONG_TOKEN_WAIT).await;
        return (
            StatusCode::UNAUTHORIZED,
            [(WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"))],
            "Unauthorized",
        )
            .into_response(cx);
    }
    respond(cx, &body, Access::Owner).await
}
