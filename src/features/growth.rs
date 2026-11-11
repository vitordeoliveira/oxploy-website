//! How oxploy grows: the stars of its GitHub repository, which are the measure of its
//! success, the downloads of its releases, and the servers that ran the install script.
//!
//! The site asks GitHub every hour, and keeps the stars and downloads of each day, as
//! GitHub only tells them as they are now. It also fills in the stars of the days
//! before it watched, from when each was given, which GitHub tells only a signed-in
//! client: with `GITHUB_TOKEN`, a token that reads public repositories.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan, Zoned};
use serde_json::Value;
use toasty::Db;
use topcoat::context::{Cx, app_context};

use crate::features::analytics::InstallFetch;

/// The repository whose stars and releases count.
pub const REPOSITORY: &str = "vitordeoliveira/oxploy";

/// How often the site asks GitHub. Without a token, GitHub answers 60 requests an hour.
const CHECK_EVERY: Duration = Duration::from_secs(60 * 60);

/// How long one request to GitHub may take.
const TIMEOUT: Duration = Duration::from_secs(20);

/// The most pages of 100 stars the history reads, so a popular repository does not
/// spend the hour's requests.
const MAX_STAR_PAGES: usize = 30;

/// The stars and downloads of one day, as the last check of the day saw them.
#[derive(Debug, toasty::Model)]
pub struct GithubDay {
    #[key]
    #[auto]
    pub id: u64,

    /// The day, such as `2026-11-11`, in the server's time zone.
    #[unique]
    pub date: String,

    /// The repository's stars.
    pub stars: i64,

    /// The downloads of the releases' archives, in all. `None` for the days before the
    /// site watched the repository, whose stars come from the history.
    pub downloads: Option<i64>,
}

/// A release, with the downloads of its archives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Its tag, such as `v0.9.0`.
    pub tag: String,
    pub downloads: i64,
    /// Whether GitHub marks it a pre-release, which is not the latest.
    pub prerelease: bool,
}

/// What the latest check saw, which the days do not keep.
#[derive(Default)]
struct Latest {
    releases: Vec<Release>,
    status: Status,
}

/// How the checks of GitHub go.
#[derive(Debug, Clone, Default)]
pub struct Status {
    /// When GitHub last answered.
    pub checked_at: Option<Timestamp>,
    /// Why the latest check failed, if it did.
    pub error: Option<String>,
    /// Why the stars of the days before the site watched are missing, if they are.
    pub no_history: Option<String>,
}

/// The watch over the repository, shared by every request.
#[derive(Clone)]
pub struct Github {
    db: Db,
    latest: Arc<Mutex<Latest>>,
}

impl Github {
    /// Starts asking GitHub, now and every hour.
    pub fn start(db: Db) -> Github {
        let github = Github {
            db,
            latest: Arc::default(),
        };
        let watcher = github.clone();
        tokio::spawn(async move {
            loop {
                let outcome = watcher.check().await;
                if let Err(error) = &outcome {
                    eprintln!("oxploy-website: GitHub could not be checked: {error}");
                }
                watcher.latest().status.error = outcome.err();
                tokio::time::sleep(CHECK_EVERY).await;
            }
        });
        github
    }

    /// Returns the watch registered on the router.
    pub fn get(cx: &Cx) -> &Github {
        app_context::<Github>(cx)
    }

    fn latest(&self) -> std::sync::MutexGuard<'_, Latest> {
        self.latest.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The releases as the latest check saw them, newest first.
    pub fn releases(&self) -> Vec<Release> {
        self.latest().releases.clone()
    }

    /// The tag of oxploy's latest release, such as `v0.9.1`, once GitHub told it.
    pub fn latest_release(&self) -> Option<String> {
        latest_release(&self.latest().releases)
    }

    /// How the checks of GitHub go.
    pub fn status(&self) -> Status {
        self.latest().status.clone()
    }

    /// Asks GitHub for the stars and the releases, and keeps them as today's.
    async fn check(&self) -> Result<(), String> {
        let (stars, releases) = tokio::task::spawn_blocking(fetch_now)
            .await
            .map_err(|error| error.to_string())??;
        let zone = TimeZone::system();
        let today = Zoned::now().with_time_zone(zone.clone()).date();
        let mut db = self.db.clone();
        let kept = GithubDay::all().exec(&mut db).await.map_err(|error| error.to_string())?;

        // The stars of the days before the site watched, until it has them. Without
        // them, today's still count.
        let watched_since = kept
            .iter()
            .filter(|day| day.downloads.is_some())
            .filter_map(|day| day.date.parse::<Date>().ok())
            .min()
            .unwrap_or(today);
        let has_history = kept.iter().any(|day| day.downloads.is_none());
        let mut no_history = None;
        if !has_history && stars > 0 {
            match tokio::task::spawn_blocking(fetch_star_history).await.map_err(|error| error.to_string())? {
                Ok(given) => {
                    for (date, stars) in history(&given, &zone, watched_since) {
                        toasty::create!(GithubDay {
                            date: date.to_string(),
                            stars,
                            downloads: None,
                        })
                        .exec(&mut db)
                        .await
                        .map_err(|error| error.to_string())?;
                    }
                }
                Err(error) => {
                    no_history = Some(format!(
                        "The stars before {watched_since} are missing: GitHub tells when each was given only with GITHUB_TOKEN ({error})."
                    ));
                }
            }
        }

        let downloads = Some(releases.iter().map(|release| release.downloads).sum());
        let today_kept = kept.into_iter().find(|day| day.date == today.to_string());
        match today_kept {
            Some(mut day) => day.update().stars(stars).downloads(downloads).exec(&mut db).await,
            None => toasty::create!(GithubDay {
                date: today.to_string(),
                stars,
                downloads,
            })
            .exec(&mut db)
            .await
            .map(|_| ()),
        }
        .map_err(|error| error.to_string())?;

        let mut latest = self.latest();
        latest.releases = releases;
        latest.status.checked_at = Some(Timestamp::now());
        latest.status.no_history = no_history;
        Ok(())
    }
}

/// Asks GitHub for something of the repository, such as `/releases`, and returns its
/// JSON. `GITHUB_TOKEN`, if set, lifts GitHub's limit of 60 requests an hour.
fn github(path: &str, accept: &str) -> Result<Value, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build()
        .into();
    let mut request = agent
        .get(&format!("https://api.github.com/repos/{REPOSITORY}{path}"))
        .header("User-Agent", "oxploy-website")
        .header("Accept", accept)
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.is_empty()
    {
        request = request.header("Authorization", &format!("Bearer {token}"));
    }
    let body = request
        .call()
        .map_err(|error| error.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}

/// The repository's stars and its releases, as they are now.
fn fetch_now() -> Result<(i64, Vec<Release>), String> {
    let repository = github("", "application/vnd.github+json")?;
    let stars = repository["stargazers_count"]
        .as_i64()
        .ok_or("GitHub gave no stargazers_count")?;
    let mut releases = Vec::new();
    for page in 1.. {
        let listed = github(&format!("/releases?per_page=100&page={page}"), "application/vnd.github+json")?;
        let count = listed.as_array().map_or(0, Vec::len);
        releases.extend(parse_releases(&listed));
        if count < 100 {
            break;
        }
    }
    Ok((stars, releases))
}

/// When each star was given, read from the stargazers, 100 to a page.
fn fetch_star_history() -> Result<Vec<Timestamp>, String> {
    let mut given = Vec::new();
    for page in 1..=MAX_STAR_PAGES {
        // This media type adds when each star was given.
        let listed = github(&format!("/stargazers?per_page=100&page={page}"), "application/vnd.github.star+json")?;
        let count = listed.as_array().map_or(0, Vec::len);
        given.extend(parse_starred_at(&listed));
        if count < 100 {
            break;
        }
    }
    Ok(given)
}

/// The published releases of GitHub's list, newest first, each with the downloads of
/// its `.tar.gz` files, leaving out their checksums.
fn parse_releases(listed: &Value) -> Vec<Release> {
    listed
        .as_array()
        .into_iter()
        .flatten()
        .filter(|release| !release["draft"].as_bool().unwrap_or(false))
        .filter_map(|release| {
            let tag = release["tag_name"].as_str()?.to_owned();
            let downloads = release["assets"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|asset| asset["name"].as_str().is_some_and(|name| name.ends_with(".tar.gz")))
                .filter_map(|asset| asset["download_count"].as_i64())
                .sum();
            let prerelease = release["prerelease"].as_bool().unwrap_or(false);
            Some(Release { tag, downloads, prerelease })
        })
        .collect()
}

/// The newest release that is not a pre-release.
fn latest_release(releases: &[Release]) -> Option<String> {
    releases.iter().find(|release| !release.prerelease).map(|release| release.tag.clone())
}

/// When each stargazer of GitHub's list gave their star.
fn parse_starred_at(listed: &Value) -> Vec<Timestamp> {
    listed
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|star| star["starred_at"].as_str()?.parse().ok())
        .collect()
}

/// The stars at the end of each day before `today` on which some were given, from when
/// each was given, in `zone`.
fn star_days(given: &[Timestamp], zone: &TimeZone, today: Date) -> Vec<(Date, i64)> {
    let mut each_day: BTreeMap<Date, i64> = BTreeMap::new();
    for at in given {
        *each_day.entry(at.to_zoned(zone.clone()).date()).or_default() += 1;
    }
    let mut stars = 0;
    each_day
        .into_iter()
        .filter(|(date, _)| *date < today)
        .map(|(date, given)| {
            stars += given;
            (date, stars)
        })
        .collect()
}

/// The days to keep from the stars' history: each day before `watched_since` on which
/// some were given, and the day before the first star, with none, which tells that the
/// history is whole even when every star came since.
fn history(given: &[Timestamp], zone: &TimeZone, watched_since: Date) -> Vec<(Date, i64)> {
    let days = star_days(given, zone, watched_since);
    let first = days.first().map_or(watched_since, |(date, _)| *date);
    let mut kept: Vec<(Date, i64)> = first.yesterday().ok().map(|before| (before, 0)).into_iter().collect();
    kept.extend(days);
    kept
}

/// A day kept: its date, its stars, and its downloads when the site counted them.
type Kept = (Date, i64, Option<i64>);

/// How oxploy grew over the last days.
pub struct Growth {
    /// Each day, oldest first.
    pub days: Vec<GrowthDay>,
    /// The stars now, and how many more than before the period, or than on the first
    /// day the site counted them, when that is in the period and their history is
    /// missing.
    pub stars: i64,
    pub stars_gained: i64,
    /// The first day the site counted the stars, when it is in the period and their
    /// history is missing.
    pub stars_since: Option<Date>,
    /// The downloads now, and how many more than before the period, or than on the
    /// first day the site counted them, when that is in the period.
    pub downloads: Option<i64>,
    pub downloads_gained: Option<i64>,
    /// The first day the site counted the downloads, when it is in the period.
    pub downloads_since: Option<Date>,
    /// The fetches of the install script, and the machines, each counted once a day.
    pub installs: usize,
    pub install_machines: usize,
    /// The time zone the days are in, such as `UTC`.
    pub time_zone: String,
}

/// One day of [`Growth`].
pub struct GrowthDay {
    pub date: Date,
    /// The stars at the end of the day, when the site knows them.
    pub stars: Option<i64>,
    /// The downloads at the end of the day, once the site counted them.
    pub downloads: Option<i64>,
    pub installs: usize,
}

impl Growth {
    /// How oxploy grew over the last `days` days, today's included, in the server's
    /// time zone.
    pub async fn of_last(db: &Db, days: i64) -> toasty::Result<Growth> {
        let zone = TimeZone::system();
        let today = Zoned::now().with_time_zone(zone.clone()).date();
        let first = today.checked_sub((days - 1).days()).unwrap_or(today);
        let since = first
            .to_zoned(zone.clone())
            .map_or(0, |start| start.timestamp().as_second());
        let kept: Vec<Kept> = GithubDay::all()
            .exec(&mut db.clone())
            .await?
            .into_iter()
            .filter_map(|day| Some((day.date.parse().ok()?, day.stars, day.downloads)))
            .collect();
        let fetches: Vec<(Date, String)> = InstallFetch::filter(InstallFetch::fields().at().ge(since))
            .exec(&mut db.clone())
            .await?
            .into_iter()
            .filter_map(|fetch| {
                let date = Timestamp::from_second(fetch.at).ok()?.to_zoned(zone.clone()).date();
                Some((date, fetch.machine))
            })
            .collect();
        let mut growth = Growth::from(kept, &fetches, first, today);
        growth.time_zone = zone.iana_name().unwrap_or("UTC").to_owned();
        Ok(growth)
    }

    /// The growth from `first` to `today`, from the days kept and the fetches of the
    /// install script, each with its day.
    fn from(mut kept: Vec<Kept>, fetches: &[(Date, String)], first: Date, today: Date) -> Growth {
        kept.sort_by_key(|(date, _, _)| *date);
        // With their history, the stars are known from the first one, and were none
        // before; without it, only from the first day the site counted them.
        let has_history = kept.iter().any(|(_, _, downloads)| downloads.is_none());
        // The last count of each kind on or before `date`.
        let stars_on = |date: Date| {
            let last = kept.iter().take_while(|(day, _, _)| *day <= date).last();
            match last {
                Some((_, stars, _)) => Some(*stars),
                None => has_history.then_some(0),
            }
        };
        let downloads_on = |date: Date| {
            kept.iter()
                .take_while(|(day, _, _)| *day <= date)
                .filter_map(|(_, _, downloads)| *downloads)
                .last()
        };
        let before = first.yesterday().unwrap_or(first);

        let mut days = Vec::new();
        let mut date = first;
        while date <= today {
            days.push(GrowthDay {
                date,
                stars: stars_on(date),
                downloads: downloads_on(date),
                installs: fetches.iter().filter(|(day, _)| *day == date).count(),
            });
            match date.tomorrow() {
                Ok(next) => date = next,
                Err(_) => break,
            }
        }

        let stars = stars_on(today).unwrap_or(0);
        let first_in_period = |counted: fn(&Kept) -> Option<i64>| {
            kept.iter()
                .filter(|day| day.0 >= first)
                .find_map(|day| counted(day).map(|count| (day.0, count)))
        };
        let (stars_baseline, stars_since) = match stars_on(before) {
            Some(counted) => (counted, None),
            None => first_in_period(|day| Some(day.1)).map_or((stars, None), |(day, counted)| (counted, Some(day))),
        };
        let downloads = downloads_on(today);
        // Before the period, or else from the first day counted in it.
        let (baseline, downloads_since) = match downloads_on(before) {
            Some(counted) => (Some(counted), None),
            None => match first_in_period(|day| day.2) {
                Some((day, counted)) => (Some(counted), Some(day)),
                None => (None, None),
            },
        };
        let machines: HashSet<&(Date, String)> = fetches.iter().collect();
        Growth {
            stars,
            stars_gained: stars - stars_baseline,
            stars_since,
            downloads,
            downloads_gained: downloads.zip(baseline).map(|(now, then)| now - then),
            downloads_since,
            installs: fetches.len(),
            install_machines: machines.len(),
            days,
            time_zone: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    #[test]
    fn counts_the_archives_of_published_releases() {
        let listed = json!([
            { "tag_name": "v0.10.0-rc.1", "draft": false, "prerelease": true, "assets": [
                { "name": "oxploy-x86_64-linux.tar.gz", "download_count": 1 },
            ] },
            { "tag_name": "v0.9.1", "draft": false, "assets": [
                { "name": "oxploy-x86_64-linux.tar.gz", "download_count": 7 },
                { "name": "oxploy-x86_64-linux.tar.gz.sha256", "download_count": 9 },
            ] },
            { "tag_name": "v1.0.0", "draft": true, "assets": [
                { "name": "oxploy-x86_64-linux.tar.gz", "download_count": 0 },
            ] },
            { "tag_name": "v0.9.0", "draft": false, "assets": [
                { "name": "oxploy-x86_64-linux.tar.gz", "download_count": 31 },
                { "name": "oxploy-aarch64-macos.tar.gz", "download_count": 2 },
            ] },
        ]);
        let releases = parse_releases(&listed);
        assert_eq!(
            releases,
            [
                Release { tag: "v0.10.0-rc.1".to_owned(), downloads: 1, prerelease: true },
                Release { tag: "v0.9.1".to_owned(), downloads: 7, prerelease: false },
                Release { tag: "v0.9.0".to_owned(), downloads: 33, prerelease: false },
            ]
        );
        // A pre-release is not the latest.
        assert_eq!(latest_release(&releases).as_deref(), Some("v0.9.1"));
        assert_eq!(latest_release(&[]), None);
    }

    #[test]
    fn rebuilds_the_stars_of_each_day_before_today() {
        let listed = json!([
            { "starred_at": "2026-11-02T10:00:00Z" },
            { "starred_at": "2026-11-02T23:00:00Z" },
            { "starred_at": "2026-11-05T08:00:00Z" },
            { "starred_at": "2026-11-11T08:00:00Z" },
            { "user": {} },
        ]);
        let given = parse_starred_at(&listed);
        assert_eq!(given.len(), 4);
        assert_eq!(
            star_days(&given, &TimeZone::UTC, date("2026-11-11")),
            [(date("2026-11-02"), 2), (date("2026-11-05"), 3)]
        );
        // A day ends at midnight where the server is.
        let sao_paulo = TimeZone::get("America/Sao_Paulo").unwrap();
        assert_eq!(
            star_days(&given, &sao_paulo, date("2026-11-11")),
            [(date("2026-11-02"), 2), (date("2026-11-05"), 3)]
        );
    }

    #[test]
    fn the_history_starts_with_none() {
        let given = parse_starred_at(&json!([
            { "starred_at": "2026-11-02T10:00:00Z" },
            { "starred_at": "2026-11-11T14:40:53Z" },
        ]));
        assert_eq!(
            history(&given, &TimeZone::UTC, date("2026-11-11")),
            [(date("2026-11-01"), 0), (date("2026-11-02"), 1)]
        );
        // Every star came since the site watched: only the none before.
        assert_eq!(history(&given[1..], &TimeZone::UTC, date("2026-11-11")), [(date("2026-11-10"), 0)]);
    }

    #[test]
    fn tells_what_the_period_gained() {
        let kept = vec![
            (date("2026-11-01"), 4, None),
            (date("2026-11-05"), 6, None),
            (date("2026-11-09"), 9, Some(40)),
            (date("2026-11-11"), 12, Some(55)),
        ];
        let fetches = vec![
            (date("2026-11-10"), "a".to_owned()),
            (date("2026-11-10"), "a".to_owned()),
            (date("2026-11-11"), "a".to_owned()),
            (date("2026-11-11"), "b".to_owned()),
        ];
        let growth = Growth::from(kept, &fetches, date("2026-11-05"), date("2026-11-11"));
        assert_eq!(growth.days.len(), 7);
        assert_eq!((growth.stars, growth.stars_gained, growth.stars_since), (12, 8, None));
        assert_eq!(growth.days[1].stars, Some(6), "a day without a check keeps the day before's");
        assert_eq!(growth.days[3].downloads, None);
        assert_eq!(growth.days[5].downloads, Some(40));
        // The site began counting downloads inside the period.
        assert_eq!((growth.downloads, growth.downloads_gained), (Some(55), Some(15)));
        assert_eq!(growth.downloads_since, Some(date("2026-11-09")));
        assert_eq!((growth.installs, growth.install_machines), (4, 3));
        assert_eq!(growth.days[5].installs, 2);
    }

    #[test]
    fn counts_from_before_the_period_when_it_can() {
        let kept = vec![(date("2026-11-01"), 4, Some(10)), (date("2026-11-11"), 5, Some(25))];
        let growth = Growth::from(kept, &[], date("2026-11-05"), date("2026-11-11"));
        assert_eq!((growth.stars_gained, growth.downloads_gained), (1, Some(15)));
        assert_eq!((growth.stars_since, growth.downloads_since), (None, None));
        assert_eq!(growth.days[0].stars, Some(4));
    }

    #[test]
    fn without_their_history_stars_count_from_the_first_check() {
        // GitHub did not tell when the stars were given, so the site only knows them
        // from the day it first asked.
        let kept = vec![(date("2026-11-09"), 3, Some(40)), (date("2026-11-11"), 5, Some(42))];
        let growth = Growth::from(kept, &[], date("2026-11-05"), date("2026-11-11"));
        assert_eq!((growth.stars, growth.stars_gained), (5, 2));
        assert_eq!(growth.stars_since, Some(date("2026-11-09")));
        assert_eq!(growth.days[0].stars, None);
        assert_eq!(growth.days[4].stars, Some(3));
    }

    #[test]
    fn with_their_history_there_were_no_stars_before_the_first() {
        let kept = vec![(date("2026-11-07"), 1, None), (date("2026-11-11"), 2, Some(9))];
        let growth = Growth::from(kept, &[], date("2026-11-05"), date("2026-11-11"));
        assert_eq!((growth.stars_gained, growth.stars_since), (2, None));
        assert_eq!(growth.days[0].stars, Some(0));
    }

    #[test]
    fn knows_nothing_before_the_first_check() {
        let growth = Growth::from(Vec::new(), &[], date("2026-11-05"), date("2026-11-11"));
        assert_eq!((growth.stars, growth.stars_gained), (0, 0));
        assert_eq!((growth.downloads, growth.downloads_gained), (None, None));
    }
}
