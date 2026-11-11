//! The `oxploy_growth` tool: oxploy's GitHub stars, downloads and installs, as the
//! admin shows them. Only the owner's server has it.

use serde::Deserialize;
use serde_json::{Value, json};
use topcoat::context::{Cx, try_app_context};

use super::common::{ToolError, parse, refuse};
use crate::db::db;
use crate::features::growth::{Github, Growth, REPOSITORY, Release};

/// The name agents call the tool by.
pub const NAME: &str = "oxploy_growth";

/// The most days the tool reports, as the admin.
const MAX_DAYS: i64 = 90;

/// The days reported when the agent asks for none.
const DEFAULT_DAYS: i64 = 30;

/// What the tool does, for the agent deciding whether to call it.
const DESCRIPTION: &str = "Reports how oxploy grows over the last days, today included, \
    as the site's admin shows it. The stars of its GitHub repository are the measure of \
    oxploy's success: their number now, how many came in the period, and each day's. \
    The downloads are those of its GitHub releases' archives, by people who install or \
    update it, which the site counts since it began watching: in all, gained in the \
    period, each day's, and each release's now. The install runs are the fetches of \
    https://oxploy.com/install.sh by curl or wget, and the machines that made them, each \
    counted once a day. Days are in the server's time zone, given as time_zone.";

/// The tool's name, description and arguments, for `tools/list`.
pub fn definition() -> Value {
    json!({
        "name": NAME,
        "title": "Report how oxploy grows",
        "description": DESCRIPTION,
        "inputSchema": {
            "type": "object",
            "properties": {
                "days": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_DAYS,
                    "default": DEFAULT_DAYS,
                    "description": "How many days to report, today included.",
                },
            },
            "additionalProperties": false,
        },
        "annotations": { "readOnlyHint": true, "openWorldHint": false },
    })
}

/// The tool's arguments, as the agent sends them.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    days: Option<i64>,
}

/// Reports the growth of the days the arguments ask for.
pub async fn call(cx: &Cx, arguments: Value) -> Result<Value, ToolError> {
    let Arguments { days } = parse(arguments)?;
    let days = days.unwrap_or(DEFAULT_DAYS);
    if !(1..=MAX_DAYS).contains(&days) {
        return Err(refuse(format!("days must be from 1 to {MAX_DAYS}, not {days}.")));
    }
    let growth = Growth::of_last(&db(cx), days).await.map_err(topcoat::Error::from)?;
    let github = try_app_context::<Github>(cx);
    let releases = github.map(Github::releases).unwrap_or_default();
    let no_history = github.and_then(|github| github.status().no_history);
    let mut report = report(&growth, &releases, days);
    if let Some(no_history) = no_history {
        report["stars"]["history"] = json!(no_history);
    }
    Ok(report)
}

/// The growth as the tool gives it.
fn report(growth: &Growth, releases: &[Release], days: i64) -> Value {
    let each_day: Vec<Value> = growth
        .days
        .iter()
        .map(|day| {
            json!({
                "date": day.date.to_string(),
                "stars": day.stars,
                "downloads": day.downloads,
                "install_runs": day.installs,
            })
        })
        .collect();
    let releases: Vec<Value> = releases
        .iter()
        .map(|release| json!({ "tag": release.tag, "downloads": release.downloads }))
        .collect();
    json!({
        "days": days,
        "time_zone": growth.time_zone,
        "repository": format!("https://github.com/{REPOSITORY}"),
        "stars": {
            "now": growth.stars,
            "gained": growth.stars_gained,
            "counted_since": growth.stars_since.map(|date| date.to_string()),
        },
        "downloads": {
            "now": growth.downloads,
            "gained": growth.downloads_gained,
            "counted_since": growth.downloads_since.map(|date| date.to_string()),
        },
        "install_runs": { "runs": growth.installs, "machines": growth.install_machines },
        "each_day": each_day,
        "releases": releases,
    })
}
