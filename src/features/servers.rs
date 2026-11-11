//! The oxploy servers that report in. Each tells the site once a day that it runs, with
//! its version and the machine it runs on, and, when its super admin chose to share it,
//! what it runs, in numbers only: how many apps, deploys and so on, never their names.
//!
//! A server is known by a random ID it made when it started for the first time, which
//! says nothing about the machine or its owner. The site keeps only a hash of it, made
//! with its secret key, and never the address the report came from.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use jiff::ToSpan;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use toasty::Db;

/// An oxploy server that reported in.
#[derive(Debug, toasty::Model)]
pub struct Server {
    #[key]
    #[auto]
    pub id: u64,

    /// The hash of the random ID the server made, with the site's secret key.
    #[unique]
    pub key: String,

    /// The first day it reported in, such as `2026-11-11`, in the site's time zone.
    pub first_seen: String,

    /// The last day it reported in.
    pub last_seen: String,

    /// The version of oxploy it ran, the last time, such as `0.10.0`.
    pub version: String,

    /// The machine it runs on, such as `x86_64-linux`.
    pub platform: String,

    /// What it runs, as the JSON of a [`Usage`], when its super admin shares it.
    pub usage: Option<String>,
}

/// A day a server reported in.
#[derive(Debug, toasty::Model)]
pub struct ServerDay {
    #[key]
    #[auto]
    pub id: u64,

    /// The day, such as `2026-11-11`, in the site's time zone.
    #[index]
    pub date: String,

    #[index]
    pub server_id: u64,
}

/// What a server sends, once a day.
#[derive(Debug, Deserialize)]
pub struct Report {
    /// The random ID it made when it started for the first time: 32 hexadecimal digits.
    pub install_id: String,
    pub version: String,
    pub platform: String,
    /// What it runs, when its super admin shares it.
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// What a server runs, in numbers. A newer oxploy may send more, which the site leaves
/// out until it knows them, and an older one fewer.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Usage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizations: Option<u32>,
    /// The apps it deploys, without the external MCP servers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apps: Option<u32>,
    /// The apps it builds with cargo, with a build script, and from GitHub releases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apps_built_with_cargo: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apps_built_with_script: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apps_deploying_releases: Option<u32>,
    /// The apps whose MCP server oxploy passes on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apps_with_mcp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_mcp_servers: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_tokens: Option<u32>,
    /// The SQLite databases in the apps' Storage tabs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub databases: Option<u32>,
    /// The domains of the apps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domains: Option<u32>,
    /// The deployments of the last 7 days, and those of them that failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployments_7d: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_deployments_7d: Option<u32>,
}

/// The largest count a report may carry, far above any real server's.
const MAX_COUNT: u32 = 1_000_000;

/// How long servers and their days are kept after they last reported in.
pub const KEPT_DAYS: i64 = 400;

impl Report {
    /// Why the report cannot be kept, if it cannot.
    pub fn problem(&self) -> Option<&'static str> {
        let id = &self.install_id;
        if id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
            return Some("install_id must be 32 lowercase hexadecimal digits.");
        }
        let fits = |text: &str, allowed: fn(u8) -> bool| {
            (1..=32).contains(&text.len()) && text.bytes().all(allowed)
        };
        if !fits(&self.version, |byte| byte.is_ascii_alphanumeric() || b".+-".contains(&byte)) {
            return Some("version must be a version of oxploy, such as 0.10.0.");
        }
        if !fits(&self.platform, |byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte)) {
            return Some("platform must be a machine, such as x86_64-linux.");
        }
        if self.usage.as_ref().is_some_and(|usage| usage.counts().any(|(_, count)| count > Some(MAX_COUNT))) {
            return Some("usage has a count too large.");
        }
        None
    }
}

impl Usage {
    /// Each count, by its name in the JSON.
    fn counts(&self) -> impl Iterator<Item = (&'static str, Option<u32>)> {
        [
            ("users", self.users),
            ("organizations", self.organizations),
            ("apps", self.apps),
            ("apps_built_with_cargo", self.apps_built_with_cargo),
            ("apps_built_with_script", self.apps_built_with_script),
            ("apps_deploying_releases", self.apps_deploying_releases),
            ("apps_with_mcp", self.apps_with_mcp),
            ("external_mcp_servers", self.external_mcp_servers),
            ("mcp_tokens", self.mcp_tokens),
            ("databases", self.databases),
            ("domains", self.domains),
            ("deployments_7d", self.deployments_7d),
            ("failed_deployments_7d", self.failed_deployments_7d),
        ]
        .into_iter()
    }
}

/// Keeps `report` as what the server `key` told on `today`.
pub async fn record(db: &Db, key: String, report: &Report, today: Date) -> toasty::Result<()> {
    let mut db = db.clone();
    let date = today.to_string();
    let usage = report
        .usage
        .as_ref()
        .and_then(|usage| serde_json::to_string(usage).ok());
    let known = Server::filter(Server::fields().key().eq(&key)).first().exec(&mut db).await?;
    let server_id = match known {
        Some(mut server) => {
            let id = server.id;
            server
                .update()
                .last_seen(date.clone())
                .version(report.version.clone())
                .platform(report.platform.clone())
                .usage(usage)
                .exec(&mut db)
                .await?;
            id
        }
        None => {
            toasty::create!(Server {
                key,
                first_seen: date.clone(),
                last_seen: date.clone(),
                version: report.version.clone(),
                platform: report.platform.clone(),
                usage,
            })
            .exec(&mut db)
            .await?
            .id
        }
    };
    let counted = ServerDay::filter(
        ServerDay::fields()
            .server_id()
            .eq(server_id)
            .and(ServerDay::fields().date().eq(&date)),
    )
    .first()
    .exec(&mut db)
    .await?;
    if counted.is_none() {
        toasty::create!(ServerDay { date, server_id }).exec(&mut db).await?;
    }
    Ok(())
}

/// Forgets the servers that have not reported in for [`KEPT_DAYS`], and the days older
/// than that.
pub async fn forget_old(db: &Db, today: Date) -> toasty::Result<()> {
    let cutoff = today.checked_sub(KEPT_DAYS.days()).unwrap_or(today).to_string();
    let mut db = db.clone();
    // Days are written as `2026-11-11`, so they sort as text.
    for server in Server::all().exec(&mut db).await? {
        if server.last_seen < cutoff {
            server.delete().exec(&mut db).await?;
        }
    }
    for day in ServerDay::all().exec(&mut db).await? {
        if day.date < cutoff {
            day.delete().exec(&mut db).await?;
        }
    }
    Ok(())
}

/// How many reports one address may send in an hour. A server sends one a day, but
/// many servers may share an address.
const REPORTS_AN_HOUR: u32 = 60;

/// The reports each address sent this hour, as the hour began for it.
static SENT: LazyLock<Mutex<HashMap<String, (Instant, u32)>>> = LazyLock::new(Mutex::default);

/// Whether `address` may send another report now, counting it if it may.
pub fn allow(address: &str) -> bool {
    let hour = Duration::from_secs(60 * 60);
    let mut sent = SENT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if sent.len() > 10_000 {
        sent.retain(|_, (since, _)| since.elapsed() < hour);
    }
    let (since, count) = sent.entry(address.to_owned()).or_insert((Instant::now(), 0));
    if since.elapsed() >= hour {
        *since = Instant::now();
        *count = 0;
    }
    *count += 1;
    *count <= REPORTS_AN_HOUR
}

/// The servers of the last days.
pub struct Servers {
    /// Each day, oldest first, with how many servers reported in.
    pub days: Vec<(Date, usize)>,
    /// The servers that reported in during the period.
    pub active: usize,
    /// The servers that reported in during the last 7 days.
    pub active_week: usize,
    /// The servers whose first report was in the period.
    pub new: usize,
    /// Of the new servers that are a week old, how many reported in a week or more after
    /// their first report, and how many there are.
    pub stayed: usize,
    pub week_old: usize,
    /// The servers of the last 7 days on each version, most first.
    pub versions: Vec<(String, usize)>,
    /// The servers of the last 7 days on each machine, most first.
    pub platforms: Vec<(String, usize)>,
    /// What the servers of the last 7 days that share it run.
    pub usage: UsageSummary,
}

/// What the servers that share their usage run.
#[derive(Debug, Default, PartialEq)]
pub struct UsageSummary {
    /// The servers that share it.
    pub sharing: usize,
    /// Their apps, in all, and the median of each server's.
    pub apps: u64,
    pub apps_median: u32,
    /// Their deployments of the last 7 days, in all, and those that failed.
    pub deployments: u64,
    pub failed_deployments: u64,
    /// For each feature, how many of the servers use it, most first.
    pub features: Vec<(&'static str, usize)>,
}

/// One server, as the report reads it: its first and last day, version, machine and
/// usage.
type Seen = (Date, Date, String, String, Option<Usage>);

impl Servers {
    /// The servers of the last `days` days, today's included.
    pub async fn of_last(db: &Db, days: i64, today: Date) -> toasty::Result<Servers> {
        let first = today.checked_sub((days - 1).days()).unwrap_or(today);
        let mut db = db.clone();
        let servers: HashMap<u64, Seen> = Server::all()
            .exec(&mut db)
            .await?
            .into_iter()
            .filter_map(|server| {
                let usage = server.usage.as_deref().and_then(|usage| serde_json::from_str(usage).ok());
                let seen = (server.first_seen.parse().ok()?, server.last_seen.parse().ok()?, server.version, server.platform, usage);
                Some((server.id, seen))
            })
            .collect();
        let first_text = first.to_string();
        let days_seen: Vec<(Date, u64)> = ServerDay::all()
            .exec(&mut db)
            .await?
            .into_iter()
            .filter(|day| day.date >= first_text)
            .filter_map(|day| Some((day.date.parse().ok()?, day.server_id)))
            .collect();
        Ok(Servers::from(&servers, &days_seen, first, today))
    }

    /// The servers from `first` to `today`, from every server known and the days each
    /// reported in.
    fn from(servers: &HashMap<u64, Seen>, days_seen: &[(Date, u64)], first: Date, today: Date) -> Servers {
        let mut by_day: BTreeMap<Date, HashSet<u64>> = BTreeMap::new();
        for (date, server) in days_seen {
            if (first..=today).contains(date) && servers.contains_key(server) {
                by_day.entry(*date).or_default().insert(*server);
            }
        }
        let mut days = Vec::new();
        let mut date = first;
        while date <= today {
            days.push((date, by_day.get(&date).map_or(0, HashSet::len)));
            match date.tomorrow() {
                Ok(next) => date = next,
                Err(_) => break,
            }
        }
        let active: HashSet<u64> = by_day.values().flatten().copied().collect();

        let week_ago = today.checked_sub(6.days()).unwrap_or(today);
        let this_week: Vec<&Seen> = servers.values().filter(|seen| seen.1 >= week_ago).collect();
        let new: Vec<&Seen> = servers.values().filter(|seen| seen.0 >= first).collect();
        // A server is a week old once 7 days passed since its first report.
        let old_enough = today.checked_sub(7.days()).unwrap_or(today);
        let week_old: Vec<&&Seen> = new.iter().filter(|seen| seen.0 <= old_enough).collect();
        let stayed = week_old
            .iter()
            .filter(|seen| seen.0.checked_add(7.days()).is_ok_and(|week_after| seen.1 >= week_after))
            .count();

        Servers {
            days,
            active: active.len(),
            active_week: this_week.len(),
            new: new.len(),
            stayed,
            week_old: week_old.len(),
            versions: tally(this_week.iter().map(|seen| seen.2.as_str())),
            platforms: tally(this_week.iter().map(|seen| seen.3.as_str())),
            usage: summarize(this_week.iter().filter_map(|seen| seen.4.as_ref())),
        }
    }
}

/// How many have each value, most first.
fn tally<'a>(values: impl Iterator<Item = &'a str>) -> Vec<(String, usize)> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for value in values {
        *counts.entry(value).or_default() += 1;
    }
    let mut listed: Vec<(String, usize)> = counts.into_iter().map(|(value, count)| (value.to_owned(), count)).collect();
    listed.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| numbers(&b.0).cmp(&numbers(&a.0))).then(a.0.cmp(&b.0)));
    listed
}

/// The numbers in `text`, in order, so `0.10.0` comes after `0.9.1`.
fn numbers(text: &str) -> Vec<u64> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// What the servers that share their usage run, from what each told.
fn summarize<'a>(shared: impl Iterator<Item = &'a Usage>) -> UsageSummary {
    let shared: Vec<&Usage> = shared.collect();
    let count = |field: fn(&Usage) -> Option<u32>| shared.iter().filter_map(|usage| field(usage)).map(u64::from).sum::<u64>();
    let using = |field: fn(&Usage) -> Option<u32>| shared.iter().filter(|usage| field(usage).is_some_and(|count| count > 0)).count();
    let mut apps: Vec<u32> = shared.iter().map(|usage| usage.apps.unwrap_or(0)).collect();
    apps.sort_unstable();
    let mut features = vec![
        ("Builds with cargo", using(|usage| usage.apps_built_with_cargo)),
        ("Builds with a script", using(|usage| usage.apps_built_with_script)),
        ("Deploys GitHub releases", using(|usage| usage.apps_deploying_releases)),
        ("Passes on an app's MCP", using(|usage| usage.apps_with_mcp)),
        ("Reaches an external MCP server", using(|usage| usage.external_mcp_servers)),
        ("Has MCP tokens", using(|usage| usage.mcp_tokens)),
        ("Keeps SQLite databases", using(|usage| usage.databases)),
        ("Serves domains", using(|usage| usage.domains)),
        ("Has more than one user", shared.iter().filter(|usage| usage.users.is_some_and(|users| users > 1)).count()),
        ("Has more than one organization", shared.iter().filter(|usage| usage.organizations.is_some_and(|organizations| organizations > 1)).count()),
    ];
    features.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    UsageSummary {
        sharing: shared.len(),
        apps: count(|usage| usage.apps),
        apps_median: apps.get(apps.len() / 2).copied().unwrap_or(0),
        deployments: count(|usage| usage.deployments_7d),
        failed_deployments: count(|usage| usage.failed_deployments_7d),
        features,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    fn report(json: &str) -> Report {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn takes_only_what_a_server_sends() {
        let good = report(r#"{"install_id":"0123456789abcdef0123456789abcdef","version":"0.10.0","platform":"x86_64-linux"}"#);
        assert_eq!(good.problem(), None);
        assert_eq!(good.usage, None);
        let with_usage = report(
            r#"{"install_id":"0123456789abcdef0123456789abcdef","version":"0.10.0-rc.1","platform":"aarch64-linux",
                "usage":{"apps":3,"domains":2,"something_newer":7}}"#,
        );
        assert_eq!(with_usage.problem(), None);
        // What the site does not know yet is left out.
        assert_eq!(serde_json::to_string(&with_usage.usage).unwrap(), r#"{"apps":3,"domains":2}"#);

        let id = |id: &str| report(&format!(r#"{{"install_id":"{id}","version":"0.10.0","platform":"x86_64-linux"}}"#));
        assert!(id("0123456789ABCDEF0123456789ABCDEF").problem().is_some());
        assert!(id("0123456789abcdef").problem().is_some());
        assert!(id("not an id at all, but 32 chars!!").problem().is_some());
        let version = |version: &str| report(&format!(r#"{{"install_id":"0123456789abcdef0123456789abcdef","version":"{version}","platform":"x86_64-linux"}}"#));
        assert!(version("").problem().is_some());
        assert!(version("<script>").problem().is_some());
        assert!(version("0.10.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0").problem().is_some());
        let huge = report(r#"{"install_id":"0123456789abcdef0123456789abcdef","version":"0.10.0","platform":"x86_64-linux","usage":{"apps":2000000}}"#);
        assert!(huge.problem().is_some());
        assert!(serde_json::from_str::<Report>(r#"{"install_id":"0123456789abcdef0123456789abcdef","version":"0.10.0","platform":"x86_64-linux","usage":{"apps":-1}}"#).is_err());
    }

    #[test]
    fn an_address_sends_only_so_many_an_hour() {
        let address = "203.0.113.77";
        for _ in 0..REPORTS_AN_HOUR {
            assert!(allow(address));
        }
        assert!(!allow(address));
        assert!(allow("203.0.113.78"));
    }

    #[test]
    fn counts_the_servers_of_the_period() {
        let usage = |apps: u32, mcp: u32| Usage {
            apps: Some(apps),
            apps_with_mcp: Some(mcp),
            deployments_7d: Some(4),
            failed_deployments_7d: Some(1),
            ..Usage::default()
        };
        let servers: HashMap<u64, Seen> = [
            // Since before the period, and still running.
            (1, (date("2026-10-01"), date("2026-11-11"), "0.10.0".to_owned(), "x86_64-linux".to_owned(), Some(usage(3, 1)))),
            // New, and still running more than a week later.
            (2, (date("2026-11-01"), date("2026-11-10"), "0.10.0".to_owned(), "x86_64-linux".to_owned(), Some(usage(1, 0)))),
            // New, and gone after two days.
            (3, (date("2026-11-02"), date("2026-11-03"), "0.9.1".to_owned(), "x86_64-linux".to_owned(), None)),
            // New this week, not a week old yet.
            (4, (date("2026-11-09"), date("2026-11-11"), "0.9.1".to_owned(), "aarch64-linux".to_owned(), None)),
        ]
        .into();
        let days_seen = vec![
            (date("2026-10-31"), 1),
            (date("2026-11-10"), 1),
            (date("2026-11-11"), 1),
            (date("2026-11-11"), 1),
            (date("2026-11-03"), 3),
            (date("2026-11-11"), 4),
            (date("2026-11-11"), 9),
        ];
        let report = Servers::from(&servers, &days_seen, date("2026-11-01"), date("2026-11-11"));
        assert_eq!(report.days.len(), 11);
        assert_eq!(report.days[10], (date("2026-11-11"), 2), "a server counts once a day, and an unknown one not at all");
        assert_eq!(report.days[2], (date("2026-11-03"), 1));
        assert_eq!(report.active, 3);
        assert_eq!(report.active_week, 3);
        assert_eq!(report.new, 3);
        assert_eq!((report.stayed, report.week_old), (1, 2));
        assert_eq!(report.versions, [("0.10.0".to_owned(), 2), ("0.9.1".to_owned(), 1)]);
        assert_eq!(report.platforms, [("x86_64-linux".to_owned(), 2), ("aarch64-linux".to_owned(), 1)]);
        let summary = &report.usage;
        assert_eq!((summary.sharing, summary.apps, summary.apps_median), (2, 4, 3));
        assert_eq!((summary.deployments, summary.failed_deployments), (8, 2));
        assert_eq!(summary.features[0], ("Passes on an app's MCP", 1));
    }

    #[test]
    fn the_newest_version_comes_first_among_equals() {
        let listed = tally(["0.9.1", "0.10.0", "0.9.10", "0.9.1"].into_iter());
        let order: Vec<&str> = listed.iter().map(|(version, _)| version.as_str()).collect();
        assert_eq!(order, ["0.9.1", "0.10.0", "0.9.10"], "the most servers first, then the newest");
    }

    #[test]
    fn nothing_shared_is_nothing_counted() {
        assert_eq!(summarize(std::iter::empty()).sharing, 0);
        let report = Servers::from(&HashMap::new(), &[], date("2026-11-05"), date("2026-11-11"));
        assert_eq!((report.active, report.new, report.week_old), (0, 0, 0));
        assert!(report.days.iter().all(|(_, count)| *count == 0));
    }
}
