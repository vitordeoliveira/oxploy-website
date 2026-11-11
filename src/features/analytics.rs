//! Visits to the site: each page someone opens, counted without cookies and without
//! keeping anyone's address. A visitor is a hash of their address and browser, made with
//! a secret key and the day, so the same person counts once a day and cannot be
//! followed from one day to the next.
//!
//! Someone who accepts the site's cookie also gets a random ID in it, which tells when
//! they come back on another day. Without their yes, nothing is stored in their
//! browser for this.

use std::collections::{BTreeMap, HashMap, HashSet};

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan, Zoned};
use sha2::{Digest, Sha256};
use toasty::Db;
use topcoat::context::{Cx, app_context};
use topcoat::cookie::{Cookie, Cookies, SameSite, cookies, time::Duration};
use topcoat::router::request::{headers, original_headers, remote_addr, uri};

/// One page someone opened.
#[derive(Debug, toasty::Model)]
pub struct PageView {
    #[key]
    #[auto]
    pub id: u64,

    /// When, in seconds since the Unix epoch.
    #[index]
    pub at: i64,

    /// The page's path, such as `/docs/installation`.
    pub path: String,

    /// Who opened it, as a hash that changes every day.
    pub visitor: String,

    /// The site the visitor came from, such as `github.com`, when it was another one.
    pub referrer: Option<String>,

    /// The kind of device: `Mobile`, `Tablet` or `Desktop`.
    pub device: Option<String>,

    /// The browser's family, such as `Firefox`.
    pub browser: Option<String>,

    /// The campaign in the link the visitor followed, its `utm_source`, such as
    /// `newsletter`.
    pub campaign: Option<String>,

    /// A hash of the ID in the visitor's cookie, when they accepted it, which stays
    /// the same from one day to the next.
    pub browser_id: Option<String>,
}

/// The secret the visitors' hashes are made with. There is one, made the first time
/// the site runs.
#[derive(Debug, toasty::Model)]
pub struct AnalyticsKey {
    #[key]
    #[auto]
    pub id: u64,

    pub key: Vec<u8>,
}

/// One fetch of the install script by a server, as
/// `curl -sSL https://oxploy.com/install.sh | sh` makes when it installs oxploy.
#[derive(Debug, toasty::Model)]
pub struct InstallFetch {
    #[key]
    #[auto]
    pub id: u64,

    /// When, in seconds since the Unix epoch.
    #[index]
    pub at: i64,

    /// Which machine fetched it, as a hash that changes every day, as a visitor's.
    pub machine: String,
}

/// How long visits are kept.
const KEPT_DAYS: i64 = 400;

/// The cookie with the visitor's choice: `yes` or `no`. It only remembers the choice, so
/// it needs no consent of its own.
const CHOICE_COOKIE: &str = "cookies";

/// The cookie with the random ID of a visitor who accepted it.
const ID_COOKIE: &str = "visitor";

/// How long the choice and the ID last, after which the site asks again: 13 months.
const COOKIE_DAYS: i64 = 395;

/// What a browser's user agent says when a program, not a person, opens the page.
const BOTS: [&str; 16] = [
    "bot", "crawl", "spider", "slurp", "curl", "wget", "python", "go-http", "headless",
    "preview", "facebookexternalhit", "monitor", "uptime", "lighthouse", "scan", "fetch",
];

/// The site's visits, shared by every request.
#[derive(Clone)]
pub struct Analytics {
    db: Db,
    key: Vec<u8>,
}

impl Analytics {
    /// Takes the key, making it the first time, and forgets the visits older than
    /// [`KEPT_DAYS`].
    pub async fn start(db: Db) -> toasty::Result<Analytics> {
        let key = match AnalyticsKey::all().first().exec(&mut db.clone()).await? {
            Some(stored) => stored.key,
            None => {
                let mut key = vec![0; 32];
                getrandom::fill(&mut key).expect("the system provides random bytes");
                toasty::create!(AnalyticsKey { key: key.clone() })
                    .exec(&mut db.clone())
                    .await?;
                key
            }
        };
        let cutoff = Timestamp::now().as_second() - KEPT_DAYS * 24 * 60 * 60;
        PageView::filter(PageView::fields().at().lt(cutoff))
            .delete()
            .exec(&mut db.clone())
            .await?;
        InstallFetch::filter(InstallFetch::fields().at().lt(cutoff))
            .delete()
            .exec(&mut db.clone())
            .await?;
        Ok(Analytics { db, key })
    }

    /// Returns the analytics registered on the router.
    pub fn get(cx: &Cx) -> &Analytics {
        app_context::<Analytics>(cx)
    }

    /// Counts the page this request opens, unless a program opened it, or the browser
    /// runtime asked for it again. It is saved on a task of its own, so the page does
    /// not wait for it.
    pub fn record(&self, cx: &Cx) {
        if original_headers(cx).contains_key("x-topcoat-runtime") {
            return;
        }
        let agent = header(cx, "user-agent");
        if is_bot(&agent) {
            return;
        }
        let referrer = referrer_host(&header(cx, "referer"), &header(cx, "host"));
        let now = Zoned::now();
        let visitor = visitor_hash(&self.key, now.date(), &client_address(cx), &agent);
        let browser_id = (choice(cx) == Some(true))
            .then(|| cookies(cx).get(ID_COOKIE))
            .flatten()
            .map(|cookie| cookie.value().to_owned())
            .filter(|id| id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .map(|id| keyed_hash(&self.key, &["cookie", &id]));
        let view = PageView {
            id: 0,
            at: now.timestamp().as_second(),
            path: uri(cx).path().to_owned(),
            visitor,
            referrer,
            device: Some(device(&agent).to_owned()),
            browser: Some(browser(&agent).to_owned()),
            campaign: uri(cx).query().and_then(campaign),
            browser_id,
        };
        let mut db = self.db.clone();
        tokio::spawn(async move {
            let saved = toasty::create!(PageView {
                at: view.at,
                path: view.path,
                visitor: view.visitor,
                referrer: view.referrer,
                device: view.device,
                browser: view.browser,
                campaign: view.campaign,
                browser_id: view.browser_id,
            })
            .exec(&mut db)
            .await;
            if let Err(error) = saved {
                eprintln!("oxploy-website: a visit could not be saved: {error}");
            }
        });
    }
}

impl Analytics {
    /// Counts a fetch of the install script, when a downloader such as curl makes it,
    /// as the install command does. A browser that opens the script, or a crawler, does
    /// not install anything, so it does not count.
    pub fn record_install(&self, cx: &Cx) {
        let agent = header(cx, "user-agent");
        if !is_installer(&agent) {
            return;
        }
        let now = Zoned::now();
        let at = now.timestamp().as_second();
        let machine = visitor_hash(&self.key, now.date(), &client_address(cx), &agent);
        let mut db = self.db.clone();
        tokio::spawn(async move {
            let saved = toasty::create!(InstallFetch { at, machine }).exec(&mut db).await;
            if let Err(error) = saved {
                eprintln!("oxploy-website: a fetch of the install script could not be saved: {error}");
            }
        });
    }
}

impl Analytics {
    /// The hash a server is kept as, from the random ID it reports in with.
    pub fn server_key(&self, install_id: &str) -> String {
        keyed_hash(&self.key, &["server", install_id])
    }
}

/// The request's header `name`, or an empty text.
fn header(cx: &Cx, name: &str) -> String {
    headers(cx)
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .trim()
        .to_owned()
}

/// The address of whoever sent the request. Behind Cloudflare, it says who that is;
/// Caddy always says who connected to it.
pub fn client_address(cx: &Cx) -> String {
    [header(cx, "cf-connecting-ip"), header(cx, "x-forwarded-for")]
        .into_iter()
        .map(|value| value.split(',').next().unwrap_or("").trim().to_owned())
        .find(|value| !value.is_empty())
        .or_else(|| remote_addr(cx).map(|address| address.ip().to_string()))
        .unwrap_or_default()
}

/// Whether a user agent is a downloader's, such as `curl/8.5.0`, which the install
/// command runs.
fn is_installer(agent: &str) -> bool {
    let agent = agent.to_ascii_lowercase();
    agent.starts_with("curl/") || agent.starts_with("wget/")
}

/// The visitor's choice about the site's cookie: `Some(true)` if they accepted it,
/// `Some(false)` if they did not, `None` if they have not chosen.
pub fn choice(cx: &Cx) -> Option<bool> {
    match cookies(cx).get(CHOICE_COOKIE)?.value() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

/// Records the visitor's choice in their browser, with a new random ID when they
/// accept, or without the ID they had when they do not. Only a route can call it,
/// before its answer starts.
pub fn choose(cx: &Cx, accept: bool) {
    let cookie = |name: &'static str, value: String| {
        Cookie::build((name, value))
            .path("/")
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax)
            .max_age(Duration::days(COOKIE_DAYS))
            .build()
    };
    let jar = cookies(cx);
    jar.add(cookie(CHOICE_COOKIE, (if accept { "yes" } else { "no" }).to_owned()));
    if accept {
        let mut id = [0; 16];
        getrandom::fill(&mut id).expect("the system provides random bytes");
        jar.add(cookie(ID_COOKIE, id.iter().map(|byte| format!("{byte:02x}")).collect()));
    } else {
        jar.remove(Cookie::build(ID_COOKIE).path("/").build());
    }
}

/// The kind of device a user agent is on.
fn device(agent: &str) -> &'static str {
    let agent = agent.to_ascii_lowercase();
    let android = agent.contains("android");
    if agent.contains("ipad") || agent.contains("tablet") || (android && !agent.contains("mobile")) {
        "Tablet"
    } else if android || agent.contains("mobi") || agent.contains("iphone") {
        "Mobile"
    } else {
        "Desktop"
    }
}

/// The family of the browser a user agent is, such as `Firefox`. The order matters:
/// Edge and Opera also say Chrome, and Chrome also says Safari.
fn browser(agent: &str) -> &'static str {
    let agent = agent.to_ascii_lowercase();
    [
        ("edg/", "Edge"),
        ("opr/", "Opera"),
        ("samsungbrowser", "Samsung Internet"),
        ("firefox/", "Firefox"),
        ("fxios", "Firefox"),
        ("chrome/", "Chrome"),
        ("crios", "Chrome"),
        ("safari/", "Safari"),
    ]
    .into_iter()
    .find(|(mark, _)| agent.contains(mark))
    .map_or("Other", |(_, name)| name)
}

/// The `utm_source` of a query, such as `newsletter`, kept to letters, digits, `.`, `-`
/// and `_`. The rest of the query is not kept, as it may hold anything.
fn campaign(query: &str) -> Option<String> {
    let value = query
        .split('&')
        .find_map(|pair| pair.strip_prefix("utm_source="))?;
    let source: String = value
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .take(50)
        .collect();
    (!source.is_empty()).then_some(source)
}

/// Whether a user agent is a program's, such as a search engine's.
fn is_bot(agent: &str) -> bool {
    let agent = agent.to_ascii_lowercase();
    agent.is_empty() || BOTS.iter().any(|bot| agent.contains(bot))
}

/// The site a visitor came from, from the `Referer` header, when it is not this one:
/// its host without `www.`.
fn referrer_host(referer: &str, own_host: &str) -> Option<String> {
    let rest = referer
        .strip_prefix("https://")
        .or_else(|| referer.strip_prefix("http://"))?;
    let host = rest.split(['/', '?', '#']).next()?.split(':').next()?.to_ascii_lowercase();
    let own = own_host.split(':').next().unwrap_or("").to_ascii_lowercase();
    let bare = |host: &str| host.strip_prefix("www.").unwrap_or(host).to_owned();
    (!host.is_empty() && bare(&host) != bare(&own)).then(|| bare(&host))
}

/// A visitor's hash for `day`: the same for one address and browser all day, and
/// another the next day.
fn visitor_hash(key: &[u8], day: Date, address: &str, agent: &str) -> String {
    keyed_hash(key, &[&day.to_string(), address, agent])
}

/// A short hash of `parts` with the site's key, which nobody without the key can tie
/// back to them.
fn keyed_hash(key: &[u8], parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key);
    for part in parts {
        hasher.update([0]);
        hasher.update(part);
    }
    hasher.finalize()[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The visits of the last days.
pub struct Report {
    /// Each day, oldest first, the ones without visits too.
    pub days: Vec<Day>,
    /// People, each counted once a day.
    pub visitors: usize,
    pub views: usize,
    /// The pages most opened, most first.
    pub pages: Vec<Row>,
    /// Where visitors came from, by the first page each opened, most first, with `None`
    /// for those who typed the address or came from a bookmark.
    pub referrers: Vec<(Option<String>, usize)>,
    /// The visitors on each kind of device, most first.
    pub devices: Vec<(String, usize)>,
    /// The visitors on each browser, most first.
    pub browsers: Vec<(String, usize)>,
    /// The visitors who followed a link with each `utm_source`, most first.
    pub campaigns: Vec<(String, usize)>,
    /// What the cookie tells, among the visitors who accepted it.
    pub accepted: Accepted,
    /// The time zone the days are in, such as `UTC`.
    pub time_zone: String,
}

/// The visits of the browsers that accepted the cookie.
pub struct Accepted {
    /// Browsers, each counted once in the whole period.
    pub browsers: usize,
    /// Of those, the ones that came on two days or more.
    pub returning: usize,
    /// Visitors, each counted once a day as in [`Report::visitors`], who had accepted.
    pub visitors: usize,
}

/// One day's visits.
pub struct Day {
    pub date: Date,
    pub views: usize,
    pub visitors: usize,
}

/// One page's visits.
pub struct Row {
    pub path: String,
    pub views: usize,
    pub visitors: usize,
}

/// How many pages and sources a report lists.
const LISTED: usize = 10;

impl Report {
    /// The visits of the last `days` days, today's included, in the server's time zone.
    pub async fn of_last(db: &Db, days: i64) -> toasty::Result<Report> {
        let zone = TimeZone::system();
        let today = Zoned::now().date();
        let first = today.checked_sub((days - 1).days()).unwrap_or(today);
        let since = first
            .to_zoned(zone.clone())
            .map_or(0, |start| start.timestamp().as_second());
        let mut views = PageView::filter(PageView::fields().at().ge(since))
            .exec(&mut db.clone())
            .await?;
        views.sort_by_key(|view| (view.at, view.id));

        let mut by_day: BTreeMap<Date, (usize, HashSet<&str>)> = BTreeMap::new();
        let mut by_page: HashMap<&str, (usize, HashSet<&str>)> = HashMap::new();
        // A visitor's later pages come from the site itself.
        let mut entered_from: HashMap<&str, Option<&str>> = HashMap::new();
        let mut everyone = HashSet::new();
        // The days each browser that accepted the cookie came on.
        let mut by_browser: HashMap<&str, HashSet<Date>> = HashMap::new();
        let mut accepting = HashSet::new();
        for view in &views {
            let Ok(at) = Timestamp::from_second(view.at) else {
                continue;
            };
            let date = at.to_zoned(zone.clone()).date();
            if let Some(browser) = &view.browser_id {
                by_browser.entry(browser).or_default().insert(date);
                accepting.insert(view.visitor.as_str());
            }
            let day = by_day.entry(date).or_default();
            day.0 += 1;
            day.1.insert(&view.visitor);
            let page = by_page.entry(&view.path).or_default();
            page.0 += 1;
            page.1.insert(&view.visitor);
            entered_from
                .entry(&view.visitor)
                .or_insert(view.referrer.as_deref());
            everyone.insert(view.visitor.as_str());
        }

        let mut days_listed = Vec::new();
        let mut date = first;
        while date <= today {
            let (views, visitors) = by_day
                .get(&date)
                .map_or((0, 0), |(views, visitors)| (*views, visitors.len()));
            days_listed.push(Day { date, views, visitors });
            match date.tomorrow() {
                Ok(next) => date = next,
                Err(_) => break,
            }
        }
        let mut pages: Vec<Row> = by_page
            .into_iter()
            .map(|(path, (views, visitors))| Row {
                path: path.to_owned(),
                views,
                visitors: visitors.len(),
            })
            .collect();
        pages.sort_by(|a, b| b.views.cmp(&a.views).then(a.path.cmp(&b.path)));
        pages.truncate(LISTED);
        let mut by_referrer: HashMap<Option<&str>, usize> = HashMap::new();
        for referrer in entered_from.into_values() {
            *by_referrer.entry(referrer).or_default() += 1;
        }
        let mut referrers: Vec<(Option<String>, usize)> = by_referrer
            .into_iter()
            .map(|(referrer, visitors)| (referrer.map(str::to_owned), visitors))
            .collect();
        referrers.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        referrers.truncate(LISTED);

        Ok(Report {
            days: days_listed,
            visitors: everyone.len(),
            views: views.len(),
            pages,
            referrers,
            devices: tally(&views, |view| view.device.as_deref()),
            browsers: tally(&views, |view| view.browser.as_deref()),
            campaigns: tally(&views, |view| view.campaign.as_deref()),
            accepted: Accepted {
                browsers: by_browser.len(),
                returning: by_browser.values().filter(|days| days.len() > 1).count(),
                visitors: accepting.len(),
            },
            time_zone: zone.iana_name().unwrap_or("UTC").to_owned(),
        })
    }
}

/// How many visitors have each value of `field`, from the first of their views that
/// has one, most first.
fn tally<'a>(views: &'a [PageView], field: impl Fn(&'a PageView) -> Option<&'a str>) -> Vec<(String, usize)> {
    let mut first: HashMap<&str, &str> = HashMap::new();
    for view in views {
        if let Some(value) = field(view) {
            first.entry(&view.visitor).or_insert(value);
        }
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for value in first.into_values() {
        *counts.entry(value).or_default() += 1;
    }
    let mut listed: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(value, visitors)| (value.to_owned(), visitors))
        .collect();
    listed.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    listed.truncate(LISTED);
    listed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_are_not_visitors() {
        assert!(is_bot(""));
        assert!(is_bot("Mozilla/5.0 (compatible; Googlebot/2.1)"));
        assert!(is_bot("curl/8.7.1"));
        assert!(!is_bot("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/131.0 Safari/537.36"));
    }

    #[test]
    fn only_downloaders_install() {
        assert!(is_installer("curl/8.5.0"));
        assert!(is_installer("Wget/1.21.4"));
        assert!(!is_installer(""));
        assert!(!is_installer("Mozilla/5.0 (compatible; Googlebot/2.1)"));
        assert!(!is_installer("Mozilla/5.0 (X11; Linux x86_64) Firefox/133.0"));
    }

    #[test]
    fn referrers_are_other_sites() {
        assert_eq!(referrer_host("https://github.com/vitordeoliveira/oxploy", "oxploy.com"), Some("github.com".to_owned()));
        assert_eq!(referrer_host("https://www.google.com/", "oxploy.com"), Some("google.com".to_owned()));
        assert_eq!(referrer_host("https://oxploy.com/docs", "oxploy.com"), None);
        assert_eq!(referrer_host("https://www.oxploy.com/", "oxploy.com"), None);
        assert_eq!(referrer_host("http://127.0.0.1:3921/", "127.0.0.1:3921"), None);
        assert_eq!(referrer_host("", "oxploy.com"), None);
    }

    #[test]
    fn devices_and_browsers_are_told_apart() {
        let iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1";
        let android = "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";
        let tablet = "Mozilla/5.0 (Linux; Android 14; SM-X710) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
        let edge = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0";
        let firefox = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14.7; rv:132.0) Gecko/20100101 Firefox/132.0";
        assert_eq!((device(iphone), browser(iphone)), ("Mobile", "Safari"));
        assert_eq!((device(android), browser(android)), ("Mobile", "Chrome"));
        assert_eq!((device(tablet), browser(tablet)), ("Tablet", "Chrome"));
        assert_eq!((device(edge), browser(edge)), ("Desktop", "Edge"));
        assert_eq!((device(firefox), browser(firefox)), ("Desktop", "Firefox"));
    }

    #[test]
    fn campaigns_keep_only_their_source() {
        assert_eq!(campaign("utm_source=Newsletter&utm_medium=email"), Some("newsletter".to_owned()));
        assert_eq!(campaign("ref=x&utm_source=reddit.com"), Some("reddit.com".to_owned()));
        assert_eq!(campaign("utm_source=%3Cscript%3E"), Some("3cscript3e".to_owned()));
        assert_eq!(campaign("utm_source="), None);
        assert_eq!(campaign("page=2"), None);
    }

    #[test]
    fn visitors_change_every_day() {
        let day = Date::constant(2026, 11, 4);
        let one = visitor_hash(b"key", day, "203.0.113.9", "Firefox");
        assert_eq!(one, visitor_hash(b"key", day, "203.0.113.9", "Firefox"));
        assert_ne!(one, visitor_hash(b"key", day.tomorrow().unwrap(), "203.0.113.9", "Firefox"));
        assert_ne!(one, visitor_hash(b"key", day, "203.0.113.10", "Firefox"));
        assert_eq!(one.len(), 16);
    }
}
