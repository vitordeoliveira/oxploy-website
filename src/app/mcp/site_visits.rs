//! The `site_visits` tool: the site's visits, as the admin shows them. Only the owner's
//! server has it.

use serde::Deserialize;
use serde_json::{Value, json};
use topcoat::context::Cx;

use super::common::{ToolError, parse, refuse};
use crate::{db::db, features::analytics::Report};

/// The name agents call the tool by.
pub const NAME: &str = "site_visits";

/// The most days the tool reports, as the admin.
const MAX_DAYS: i64 = 90;

/// The days reported when the agent asks for none.
const DEFAULT_DAYS: i64 = 30;

/// What the tool does, for the agent deciding whether to call it.
const DESCRIPTION: &str = "Reports the visits to oxploy's site, https://oxploy.com, \
    over the last days, today included, as its admin shows them. The site counts them \
    itself, without cookies: a visitor is a person on one day, so someone who comes on \
    two days counts twice, and programs such as search engines are left out. It gives \
    the totals, each day, the 10 pages most opened, and the 10 sources, devices, \
    browsers and campaigns (the links' utm_source, such as newsletter) with the most \
    visitors. A source is the other site a visitor came from, by their first page, or \
    direct for a typed address or a bookmark. The cookie part counts only the visitors \
    who accepted the cookie: how many browsers, and how many of them came back on \
    another day. Days are in the server's time zone, given as time_zone.";

/// The tool's name, description and arguments, for `tools/list`.
pub fn definition() -> Value {
    json!({
        "name": NAME,
        "title": "Report the site's visits",
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

/// Reports the visits of the days the arguments ask for.
pub async fn call(cx: &Cx, arguments: Value) -> Result<Value, ToolError> {
    let Arguments { days } = parse(arguments)?;
    let days = days.unwrap_or(DEFAULT_DAYS);
    if !(1..=MAX_DAYS).contains(&days) {
        return Err(refuse(format!("days must be from 1 to {MAX_DAYS}, not {days}.")));
    }
    let report = Report::of_last(&db(cx), days).await.map_err(topcoat::Error::from)?;
    Ok(visits(&report, days))
}

/// The report as the tool gives it.
fn visits(report: &Report, days: i64) -> Value {
    let each = |rows: &[(String, usize)], key: &str| -> Vec<Value> {
        rows.iter()
            .map(|(value, visitors)| json!({ key: value, "visitors": visitors }))
            .collect()
    };
    let each_day: Vec<_> = report
        .days
        .iter()
        .map(|day| json!({ "date": day.date.to_string(), "views": day.views, "visitors": day.visitors }))
        .collect();
    let pages: Vec<_> = report
        .pages
        .iter()
        .map(|page| json!({ "path": page.path, "views": page.views, "visitors": page.visitors }))
        .collect();
    let sources: Vec<_> = report
        .referrers
        .iter()
        .map(|(host, visitors)| json!({ "source": host.as_deref().unwrap_or("direct"), "visitors": visitors }))
        .collect();
    json!({
        "days": days,
        "time_zone": report.time_zone,
        "visitors": report.visitors,
        "views": report.views,
        "each_day": each_day,
        "pages": pages,
        "sources": sources,
        "devices": each(&report.devices, "device"),
        "browsers": each(&report.browsers, "browser"),
        "campaigns": each(&report.campaigns, "utm_source"),
        "cookie": {
            "visitors_who_accepted": report.accepted.visitors,
            "browsers": report.accepted.browsers,
            "browsers_that_came_back": report.accepted.returning,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::analytics::{Accepted, Day, Row};

    #[test]
    fn reports_every_part_of_the_admin() {
        let report = Report {
            days: vec![Day {
                date: "2026-11-05".parse().unwrap(),
                views: 12,
                visitors: 5,
            }],
            visitors: 5,
            views: 12,
            pages: vec![Row {
                path: "/docs/installation".to_owned(),
                views: 8,
                visitors: 5,
            }],
            referrers: vec![(Some("github.com".to_owned()), 3), (None, 2)],
            devices: vec![("Desktop".to_owned(), 4)],
            browsers: vec![("Firefox".to_owned(), 3)],
            campaigns: vec![("newsletter".to_owned(), 3)],
            accepted: Accepted {
                browsers: 2,
                returning: 1,
                visitors: 2,
            },
            time_zone: "America/Sao_Paulo".to_owned(),
        };
        let visits = visits(&report, 7);
        assert_eq!(visits["time_zone"], "America/Sao_Paulo");
        assert_eq!(visits["each_day"][0]["date"], "2026-11-05");
        assert_eq!(visits["pages"][0]["path"], "/docs/installation");
        assert_eq!(visits["sources"][1]["source"], "direct");
        assert_eq!(visits["campaigns"][0]["utm_source"], "newsletter");
        assert_eq!(visits["cookie"]["browsers_that_came_back"], 1);
    }
}
