//! What oxploy servers send the site, under `/api`.

use jiff::Zoned;
use topcoat::{
    Result,
    context::Cx,
    router::{
        StatusCode,
        request::Bytes,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::db::db;
use crate::features::analytics::{Analytics, client_address};
use crate::features::servers::{self, Report};

/// The largest report accepted, in bytes.
pub const MAX_REPORT: usize = 16 * 1024;

/// A server's daily report: that it runs, its version, and what it runs when its super
/// admin shares it. Answers 204 once kept, 400 for a report the site cannot read, and
/// 429 when its address sent too many this hour.
#[route(POST "./ping")]
async fn ping(cx: &Cx, body: Bytes) -> Result<Response> {
    if !servers::allow(&client_address(cx)) {
        return (StatusCode::TOO_MANY_REQUESTS, "Too many reports from this address. Try again in an hour.").into_response(cx);
    }
    let report: Report = match serde_json::from_slice(&body) {
        Ok(report) => report,
        Err(_) => return (StatusCode::BAD_REQUEST, "The body is not a report of oxploy.").into_response(cx),
    };
    if let Some(problem) = report.problem() {
        return (StatusCode::BAD_REQUEST, problem).into_response(cx);
    }
    let key = Analytics::get(cx).server_key(&report.install_id);
    servers::record(&db(cx), key, &report, Zoned::now().date()).await?;
    StatusCode::NO_CONTENT.into_response(cx)
}
