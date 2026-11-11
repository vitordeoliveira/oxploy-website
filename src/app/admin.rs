//! The admin's page, at `/admin`: a form to sign in, and once signed in, how oxploy
//! grows, by its GitHub stars, its downloads and its installs, the oxploy servers that
//! report in, and the site's visits of the last days.

pub mod mcp;

use jiff::civil::Date;
use jiff::{Timestamp, Zoned};
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{
        content::Form,
        error::{SeeOther, see_other},
        page, query_params, route,
    },
    view::{View, attributes, component, view},
};

use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::input::input;
use crate::db::db;
use crate::features::admin;
use crate::features::analytics::{Accepted, Report};
use crate::features::growth::{Github, Growth, REPOSITORY, Release, Status};
use crate::features::servers::Servers;

/// How many releases the growth lists, newest first.
const LISTED_RELEASES: usize = 10;

/// The periods the page shows, in days.
const PERIODS: [i64; 3] = [7, 30, 90];

/// What `/admin` was asked for: how many days, and whether the last sign-in was wrong.
#[query_params(error = bad_request)]
struct Asked {
    days: Option<i64>,
    wrong: Option<String>,
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let asked = query_params::<Asked>(cx)?;
    let days = asked.days.filter(|days| PERIODS.contains(days)).unwrap_or(30);
    let signed_in = admin::signed_in(cx).await?;
    let report = if signed_in {
        Some(Report::of_last(&db(cx), days).await?)
    } else {
        None
    };
    let growth = if signed_in {
        Some(Growth::of_last(&db(cx), days).await?)
    } else {
        None
    };
    let servers = if signed_in {
        Some(Servers::of_last(&db(cx), days, Zoned::now().date()).await?)
    } else {
        None
    };
    let github = Github::get(cx);
    let releases = github.releases();
    let status = github.status();

    Ok(view! {
        <main class="mx-auto w-full max-w-6xl px-4 py-10 sm:px-6">
            if let (Some(report), Some(growth), Some(servers)) = (&report, &growth, &servers) {
                dashboard(
                    report: report,
                    growth: growth,
                    servers: servers,
                    releases: &releases,
                    status: &status,
                    days: days
                )
            } else {
                sign_in_form(set_up: admin::is_set_up(), wrong: asked.wrong.is_some())
            }
        </main>
    })
}

/// The form that signs the admin in, or what to set up first.
#[component]
async fn sign_in_form(set_up: bool, wrong: bool) -> Result<impl View> {
    Ok(view! {
        <div class="mx-auto flex max-w-sm flex-col gap-6 py-10">
            <div class="flex flex-col gap-2">
                <h1 class="text-2xl font-semibold tracking-tight">"Admin"</h1>
                <p class="text-sm text-muted-foreground">"The site's visits, for its admin."</p>
            </div>
            if set_up {
                <form method="post" action="/admin/sign-in" class="flex flex-col gap-4">
                    <label class="flex flex-col gap-2 text-sm font-medium">
                        "Name"
                        input(attrs: attributes! { name="name" autocomplete="username" required="" })
                    </label>
                    <label class="flex flex-col gap-2 text-sm font-medium">
                        "Password"
                        input(attrs: attributes! { name="password" type="password" autocomplete="current-password" required="" })
                    </label>
                    if wrong {
                        <p class="flex items-center gap-2 text-sm text-destructive">
                            icon(data: iconify_icon!("lucide:circle-alert"), size: 16, attrs: attributes! { class="shrink-0" })
                            "Wrong name or password."
                        </p>
                    }
                    button(attrs: attributes! { type="submit" }, "Sign in")
                </form>
            } else {
                <p class="rounded-lg border border-border bg-card p-4 text-sm text-muted-foreground">
                    "Set " <code class="font-mono text-foreground">"ADMIN_USERNAME"</code> " and "
                    <code class="font-mono text-foreground">"ADMIN_PASSWORD"</code>
                    " in the site's environment, such as its service's Environment tab in oxploy, to sign in here."
                </p>
            }
        </div>
    })
}

/// The visits of the last `days` days.
#[component]
async fn dashboard(
    report: &Report,
    growth: &Growth,
    servers: &Servers,
    releases: &[Release],
    status: &Status,
    days: i64,
) -> Result<impl View> {
    let per_visitor = if report.visitors == 0 {
        "0".to_owned()
    } else {
        format!("{:.1}", report.views as f64 / report.visitors as f64)
    };

    Ok(view! {
        <div class="flex flex-col gap-8">
            <div class="flex flex-wrap items-end justify-between gap-4">
                <div class="flex flex-col gap-1">
                    <h1 class="text-2xl font-semibold tracking-tight">"Analytics"</h1>
                    <p class="max-w-2xl text-sm text-muted-foreground">
                        "How oxploy grows, and who reads this site. Days are in " (&report.time_zone) "."
                    </p>
                </div>
                <div class="flex items-center gap-2">
                    <nav class="flex rounded-lg border border-border p-0.5 text-sm" aria-label="Period">
                        #[key(*period)]
                        for period in &PERIODS {
                            <a
                                href=(format!("/admin?days={period}"))
                                class=(if *period == days { "rounded-md bg-foreground/10 px-3 py-1.5 font-medium" } else { "rounded-md px-3 py-1.5 text-muted-foreground hover:text-foreground" })
                                aria-current=((*period == days).then_some("page"))
                            >
                                (period.to_string()) " days"
                            </a>
                        }
                    </nav>
                    <form method="post" action="/admin/sign-out">
                        button(variant: ButtonVariant::Ghost, size: ButtonSize::Sm, attrs: attributes! { type="submit" }, "Sign out")
                    </form>
                </div>
            </div>

            growth_section(growth: growth, releases: releases, status: status)

            servers_section(servers: servers, installs: growth.installs)

            <div class="flex flex-col gap-1 border-t border-border pt-8">
                <h2 class="text-lg font-semibold tracking-tight">"Visits"</h2>
                <p class="max-w-2xl text-sm text-muted-foreground">
                    "The people who opened the site's pages, each counted once a day, without keeping their address, whether they accepted the cookie or not. Programs, such as search engines, are left out."
                </p>
            </div>

            <div class="grid gap-4 sm:grid-cols-3">
                stat(label: "Visitors", value: grouped(report.visitors), note: "People, once a day each")
                stat(label: "Page views", value: grouped(report.views), note: "Pages opened")
                stat(label: "Pages per visitor", value: per_visitor.clone(), note: "Page views for each visitor")
            </div>

            daily_chart(report: report)

            <div class="grid gap-4 lg:grid-cols-2">
                <section class="flex flex-col gap-3 rounded-xl border border-border bg-card p-5">
                    <h2 class="font-semibold">"Pages"</h2>
                    if report.pages.is_empty() {
                        <p class="text-sm text-muted-foreground">"No page was opened in these days."</p>
                    } else {
                        <div class="flex justify-between px-3 text-xs text-muted-foreground">
                            <span>"Page"</span>
                            <span>"Views · visitors"</span>
                        </div>
                        <ol class="flex flex-col gap-1">
                            #[key(row.path.clone())]
                            for row in &report.pages {
                                share_row(
                                    label: &row.path,
                                    value: format!("{} · {}", grouped(row.views), grouped(row.visitors)),
                                    share: share(row.views, report.pages[0].views),
                                    monospace: true
                                )
                            }
                        </ol>
                    }
                </section>
                <section class="flex flex-col gap-3 rounded-xl border border-border bg-card p-5">
                    <h2 class="font-semibold">"Sources"</h2>
                    if report.referrers.is_empty() {
                        <p class="text-sm text-muted-foreground">"No one came in these days."</p>
                    } else {
                        <div class="flex justify-between px-3 text-xs text-muted-foreground">
                            <span>"Where visitors came from"</span>
                            <span>"Visitors"</span>
                        </div>
                        <ol class="flex flex-col gap-1">
                            #[key(source.clone().unwrap_or_default())]
                            for (source, visitors) in &report.referrers {
                                share_row(
                                    label: source.as_deref().unwrap_or("Direct, or a bookmark"),
                                    value: grouped(*visitors),
                                    share: share(*visitors, report.referrers[0].1),
                                    monospace: false
                                )
                            }
                        </ol>
                    }
                </section>
            </div>

            <div class="grid gap-4 lg:grid-cols-3">
                tally_card(title: "Devices", column: "Device", rows: &report.devices, empty: "No one came in these days.")
                tally_card(title: "Browsers", column: "Browser", rows: &report.browsers, empty: "No one came in these days.")
                tally_card(
                    title: "Campaigns",
                    column: "utm_source",
                    rows: &report.campaigns,
                    empty: "No one followed a link with a utm_source in these days. Add one to the links you share, such as ?utm_source=newsletter, to see them here."
                )
            </div>

            accepted_card(accepted: &report.accepted, visitors: report.visitors)
        </div>
    })
}

/// How oxploy grows: its GitHub stars, which measure its success, the downloads of its
/// releases, and the machines that ran the install script.
#[component]
async fn growth_section(growth: &Growth, releases: &[Release], status: &Status) -> Result<impl View> {
    let stars_note = match growth.stars_since {
        Some(since) => format!("{} since {}, when the count began", signed(growth.stars_gained), short_date(since)),
        None => format!("{} in these days", signed(growth.stars_gained)),
    };
    let (downloads, downloads_note) = match (growth.downloads, growth.downloads_gained, growth.downloads_since) {
        (Some(downloads), Some(gained), Some(since)) => (grouped(downloads as usize), format!("{} since {}, when the count began", signed(gained), short_date(since))),
        (Some(downloads), Some(gained), None) => (grouped(downloads as usize), format!("{} in these days", signed(gained))),
        (Some(downloads), None, _) => (grouped(downloads as usize), "Of the releases' archives".to_owned()),
        (None, _, _) => ("–".to_owned(), "Not counted yet".to_owned()),
    };
    let installs_note = format!("{} machines, once a day each", grouped(growth.install_machines));
    let mut checked = match (&status.error, status.checked_at) {
        (Some(error), Some(at)) => format!("GitHub answered {}, but not the last time: {error}.", ago(at)),
        (Some(error), None) => format!("GitHub could not be asked: {error}."),
        (None, Some(at)) => format!("GitHub answered {}.", ago(at)),
        (None, None) => "Asking GitHub…".to_owned(),
    };
    if let Some(no_history) = &status.no_history {
        checked = format!("{checked} {no_history}");
    }
    let stars: Vec<(Date, Option<i64>)> = growth.days.iter().map(|day| (day.date, day.stars)).collect();
    let downloads_each_day: Vec<(Date, Option<i64>)> = growth.days.iter().map(|day| (day.date, day.downloads)).collect();
    let installs: Vec<(Date, usize)> = growth.days.iter().map(|day| (day.date, day.installs)).collect();
    let releases = &releases[..releases.len().min(LISTED_RELEASES)];
    let most = releases.iter().map(|release| release.downloads).max().unwrap_or(0).max(0) as usize;

    Ok(view! {
        <section class="flex flex-col gap-4">
            <div class="flex flex-col gap-1">
                <h2 class="text-lg font-semibold tracking-tight">"Growth"</h2>
                <p class="max-w-2xl text-sm text-muted-foreground">
                    "The stars of " <a class="font-medium text-foreground underline decoration-foreground/30 underline-offset-4" href=(format!("https://github.com/{REPOSITORY}"))>(REPOSITORY)</a>
                    ", the measure of oxploy's success, the downloads of its releases from GitHub, and the machines that ran the install script. " (checked)
                </p>
            </div>
            <div class="grid gap-4 sm:grid-cols-3">
                stat(label: "GitHub stars", value: grouped(growth.stars.max(0) as usize), note: &stars_note)
                stat(label: "Release downloads", value: downloads, note: &downloads_note)
                stat(label: "Install script runs", value: grouped(growth.installs), note: &installs_note)
            </div>
            <div class="grid gap-4 lg:grid-cols-2">
                line_chart(title: "Stars", values: stars)
                line_chart(title: "Release downloads", values: downloads_each_day)
            </div>
            <div class="grid gap-4 lg:grid-cols-2">
                count_chart(title: "Install script runs", values: installs)
                <section class="flex flex-col gap-3 rounded-xl border border-border bg-card p-5">
                    <h2 class="font-semibold">"Latest releases"</h2>
                    if releases.is_empty() {
                        <p class="text-sm text-muted-foreground">"GitHub has not told of any release yet."</p>
                    } else {
                        <div class="flex justify-between px-3 text-xs text-muted-foreground">
                            <span>"Release"</span>
                            <span>"Downloads of its archives"</span>
                        </div>
                        <ol class="flex flex-col gap-1">
                            #[key(release.tag.clone())]
                            for release in releases {
                                share_row(
                                    label: &release.tag,
                                    value: grouped(release.downloads.max(0) as usize),
                                    share: share(release.downloads.max(0) as usize, most),
                                    monospace: true
                                )
                            }
                        </ol>
                    }
                </section>
            </div>
        </section>
    })
}

/// The oxploy servers that report in: how many run, how many are new and stay, their
/// versions and machines, and what those that share it run.
#[component]
async fn servers_section(servers: &Servers, installs: usize) -> Result<impl View> {
    let usage = &servers.usage;
    let each_day = servers.days.clone();
    let active_note = format!("Reported in during the last 7 days, and {} in these days", grouped(servers.active));
    let new_note = format!("First reported in these days, when the install script ran {} times", grouped(installs));
    let stayed_note = if servers.week_old == 0 {
        "Of the new servers a week old, when there are some".to_owned()
    } else {
        format!("{} of the {} new servers a week old reported in a week later", grouped(servers.stayed), grouped(servers.week_old))
    };
    let failed_note = format!(
        "{} failed, {}",
        grouped(usage.failed_deployments as usize),
        percent(usage.failed_deployments as usize, usage.deployments as usize)
    );

    Ok(view! {
        <section class="flex flex-col gap-4 border-t border-border pt-8">
            <div class="flex flex-col gap-1">
                <h2 class="text-lg font-semibold tracking-tight">"Servers"</h2>
                <p class="max-w-2xl text-sm text-muted-foreground">
                    "The oxploy servers that report in, once a day each, by a random ID that says nothing about the machine or its owner. One that turned reporting off, or cannot reach this site, is not counted."
                </p>
            </div>
            <div class="grid gap-4 sm:grid-cols-3">
                stat(label: "Running servers", value: grouped(servers.active_week), note: &active_note)
                stat(label: "New servers", value: grouped(servers.new), note: &new_note)
                stat(label: "Still running a week later", value: percent(servers.stayed, servers.week_old), note: &stayed_note)
            </div>
            count_chart(title: "Servers each day", values: each_day)
            <div class="grid gap-4 lg:grid-cols-2">
                tally_card(
                    title: "Versions",
                    column: "Version, in the last 7 days",
                    counted: Some("Servers"),
                    rows: &servers.versions,
                    empty: "No server reported in during the last 7 days."
                )
                tally_card(
                    title: "Machines",
                    column: "Machine, in the last 7 days",
                    counted: Some("Servers"),
                    rows: &servers.platforms,
                    empty: "No server reported in during the last 7 days."
                )
            </div>
            <section class="flex flex-col gap-4 rounded-xl border border-border bg-card p-5">
                <div class="flex flex-col gap-1">
                    <h2 class="font-semibold">"What they run"</h2>
                    <p class="text-sm text-muted-foreground">
                        "From the servers of the last 7 days whose super admin shares it, in Settings: numbers only, never names. They say less the fewer share."
                    </p>
                </div>
                if usage.sharing == 0 {
                    <p class="text-sm text-muted-foreground">"No server shares what it runs yet."</p>
                } else {
                    <div class="grid gap-4 sm:grid-cols-3">
                        measure(
                            label: "Sharing",
                            value: grouped(usage.sharing),
                            note: format!("{} of the servers of the last 7 days", percent(usage.sharing, servers.active_week))
                        )
                        measure(
                            label: "Apps",
                            value: grouped(usage.apps as usize),
                            note: format!("{} on the median server", usage.apps_median)
                        )
                        measure(
                            label: "Deployments in 7 days",
                            value: grouped(usage.deployments as usize),
                            note: failed_note
                        )
                    </div>
                    <div class="flex justify-between px-3 text-xs text-muted-foreground">
                        <span>"What a server does"</span>
                        <span>"Servers"</span>
                    </div>
                    <ol class="flex flex-col gap-1">
                        #[key(*feature)]
                        for (feature, count) in &usage.features {
                            share_row(
                                label: feature,
                                value: format!("{} · {}", grouped(*count), percent(*count, usage.sharing)),
                                share: share(*count, usage.sharing),
                                monospace: false
                            )
                        }
                    </ol>
                }
            </section>
        </section>
    })
}

/// A count that only grows, such as the stars, as a line over the days, with the
/// numbers in a table under it. A day without a count breaks the line.
#[component]
async fn line_chart(title: &str, values: Vec<(Date, Option<i64>)>) -> Result<impl View> {
    let peak = values.iter().filter_map(|(_, value)| *value).max().unwrap_or(0).max(0) as usize;
    let top = nice_ceiling(peak);
    let count = values.len();
    let x = |index: usize| if count <= 1 { 50.0 } else { index as f64 * 100.0 / (count - 1) as f64 };
    let y = |value: i64| 100.0 - share(value.max(0) as usize, top) * 100.0;
    // The runs of days with a count, each a line and the area under it.
    let mut runs: Vec<Vec<(f64, f64)>> = vec![Vec::new()];
    for (index, (_, value)) in values.iter().enumerate() {
        match value {
            Some(value) => runs.last_mut().expect("there is a run").push((x(index), y(*value))),
            None => runs.push(Vec::new()),
        }
    }
    let shapes: Vec<(String, String)> = runs
        .into_iter()
        .filter(|run| !run.is_empty())
        .map(|mut run| {
            // A single day is a short level line, which a line of one point is not.
            if run.len() == 1 {
                let (x, y) = run[0];
                run = vec![(x - 0.5, y), (x + 0.5, y)];
            }
            let line: Vec<String> = run.iter().map(|(x, y)| format!("{x:.2},{y:.2}")).collect();
            let area = format!("{:.2},100 {} {:.2},100", run[0].0, line.join(" "), run[run.len() - 1].0);
            (line.join(" "), area)
        })
        .collect();
    let last = count.saturating_sub(1);
    let labeled = move |index: usize| index == 0 || index == last || index == last / 2;

    Ok(view! {
        <section class="flex flex-col gap-4 rounded-xl border border-border bg-card p-5">
            <h2 class="font-semibold">(title)</h2>
            <div class="flex gap-3">
                <div class="flex h-40 flex-col justify-between text-right text-xs text-muted-foreground tabular-nums">
                    <span class="-translate-y-1/2">(grouped(top))</span>
                    <span class="-translate-y-1/2">(grouped(top / 2))</span>
                    <span class="translate-y-1/2">"0"</span>
                </div>
                <div class="flex min-w-0 flex-1 flex-col gap-2">
                    <div class="relative h-40">
                        <div class="absolute inset-x-0 top-0 border-t border-border/70"></div>
                        <div class="absolute inset-x-0 top-1/2 border-t border-border/70"></div>
                        <div class="absolute inset-x-0 bottom-0 border-t border-border"></div>
                        <svg class="absolute inset-0 size-full overflow-visible" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
                            #[key(index)]
                            for (index, (line, area)) in shapes.iter().enumerate() {
                                <polygon class="fill-chart-1/15" points=(area)></polygon>
                                <polyline class="fill-none stroke-chart-1" stroke-width="2" stroke-linejoin="round" vector-effect="non-scaling-stroke" points=(line)></polyline>
                            }
                        </svg>
                    </div>
                    <div class="flex text-xs text-muted-foreground">
                        #[key(index)]
                        for (index, (date, _)) in values.iter().enumerate() {
                            <span class="flex-1 text-center whitespace-nowrap">
                                if labeled(index) { (short_date(*date)) }
                            </span>
                        }
                    </div>
                </div>
            </div>
            <details class="text-sm">
                <summary class="cursor-pointer text-muted-foreground hover:text-foreground">"Show the numbers"</summary>
                <table class="mt-3 w-full text-left tabular-nums">
                    <thead class="text-xs text-muted-foreground">
                        <tr><th class="py-1.5 font-medium">"Day"</th><th class="py-1.5 text-right font-medium">(title)</th></tr>
                    </thead>
                    <tbody class="divide-y divide-border">
                        #[key(date.to_string())]
                        for (date, value) in values.iter().rev() {
                            <tr>
                                <td class="py-1.5">(long_date(*date))</td>
                                <td class="py-1.5 text-right">(value.map_or("–".to_owned(), |value| grouped(value.max(0) as usize)))</td>
                            </tr>
                        }
                    </tbody>
                </table>
            </details>
        </section>
    })
}

/// A count of each day, such as the install script's runs, as bars, with the numbers
/// in a table under them.
#[component]
async fn count_chart(title: &str, values: Vec<(Date, usize)>) -> Result<impl View> {
    let top = nice_ceiling(values.iter().map(|(_, count)| *count).max().unwrap_or(0));
    let height = move |count: usize| format!("height: {:.1}%", share(count, top) * 100.0);
    let last = values.len().saturating_sub(1);
    let labeled = move |index: usize| index == 0 || index == last || index == last / 2;

    Ok(view! {
        <section class="flex flex-col gap-4 rounded-xl border border-border bg-card p-5">
            <h2 class="font-semibold">(title)</h2>
            <div class="flex gap-3">
                <div class="flex h-40 flex-col justify-between text-right text-xs text-muted-foreground tabular-nums">
                    <span class="-translate-y-1/2">(grouped(top))</span>
                    <span class="-translate-y-1/2">(grouped(top / 2))</span>
                    <span class="translate-y-1/2">"0"</span>
                </div>
                <div class="flex min-w-0 flex-1 flex-col gap-2">
                    <div class="relative h-40">
                        <div class="absolute inset-x-0 top-0 border-t border-border/70"></div>
                        <div class="absolute inset-x-0 top-1/2 border-t border-border/70"></div>
                        <div class="absolute inset-0 flex items-end gap-0.5 border-b border-border">
                            #[key(date.to_string())]
                            for (date, count) in &values {
                                <div class="flex h-full flex-1 items-end justify-center rounded-t hover:bg-foreground/5" title=(format!("{}: {count}", long_date(*date)))>
                                    <span class="w-full max-w-4 rounded-t bg-chart-1" style=(height(*count))></span>
                                </div>
                            }
                        </div>
                    </div>
                    <div class="flex text-xs text-muted-foreground">
                        #[key(index)]
                        for (index, (date, _)) in values.iter().enumerate() {
                            <span class="flex-1 text-center whitespace-nowrap">
                                if labeled(index) { (short_date(*date)) }
                            </span>
                        }
                    </div>
                </div>
            </div>
        </section>
    })
}

/// A list of how many visitors each value has, such as each browser.
#[component]
async fn tally_card(title: &str, column: &str, #[default] counted: Option<&str>, rows: &[(String, usize)], empty: &str) -> Result<impl View> {
    Ok(view! {
        <section class="flex flex-col gap-3 rounded-xl border border-border bg-card p-5">
            <h2 class="font-semibold">(title)</h2>
            if rows.is_empty() {
                <p class="text-sm text-muted-foreground">(empty)</p>
            } else {
                <div class="flex justify-between px-3 text-xs text-muted-foreground">
                    <span>(column)</span>
                    <span>(counted.unwrap_or("Visitors"))</span>
                </div>
                <ol class="flex flex-col gap-1">
                    #[key(value.clone())]
                    for (value, visitors) in rows {
                        share_row(
                            label: value,
                            value: grouped(*visitors),
                            share: share(*visitors, rows[0].1),
                            monospace: false
                        )
                    }
                </ol>
            }
        </section>
    })
}

/// What the cookie tells, among the visitors who accepted it.
#[component]
async fn accepted_card(accepted: &Accepted, visitors: usize) -> Result<impl View> {
    Ok(view! {
        <section class="flex flex-col gap-4 rounded-xl border border-border bg-card p-5">
            <div class="flex flex-col gap-1">
                <h2 class="font-semibold">"Visitors who accepted the cookie"</h2>
                <p class="text-sm text-muted-foreground">
                    "The cookie tells when the same browser comes back on another day. These numbers count only the visitors who accepted it, so they say less the fewer accept."
                </p>
            </div>
            <div class="grid gap-4 sm:grid-cols-3">
                measure(
                    label: "Accepted",
                    value: percent(accepted.visitors, visitors),
                    note: format!("{} of the {} visitors", grouped(accepted.visitors), grouped(visitors))
                )
                measure(
                    label: "Browsers",
                    value: grouped(accepted.browsers),
                    note: "Each counted once in these days".to_owned()
                )
                measure(
                    label: "Came back",
                    value: percent(accepted.returning, accepted.browsers),
                    note: format!("{} came on two days or more", grouped(accepted.returning))
                )
            </div>
        </section>
    })
}

/// A number inside a card, with what it counts.
#[component]
async fn measure(label: &str, value: String, note: String) -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-col gap-1">
            <span class="text-sm text-muted-foreground">(label)</span>
            <span class="text-2xl font-semibold tracking-tight tabular-nums">(value)</span>
            <span class="text-xs text-muted-foreground">(note)</span>
        </div>
    })
}

/// One number of the period, with what it counts.
#[component]
async fn stat(label: &str, value: String, note: &str) -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-col gap-1 rounded-xl border border-border bg-card p-5">
            <span class="text-sm text-muted-foreground">(label)</span>
            <span class="text-3xl font-semibold tracking-tight tabular-nums">(value)</span>
            <span class="text-xs text-muted-foreground">(note)</span>
        </div>
    })
}

/// A line of a list, with a bar behind it as long as its share of the first line's.
#[component]
async fn share_row(label: &str, value: String, share: f64, monospace: bool) -> Result<impl View> {
    Ok(view! {
        <li class="relative flex items-center justify-between gap-3 overflow-hidden rounded-md px-3 py-2 text-sm">
            <span
                class="absolute inset-y-0 left-0 rounded-md bg-chart-1/15"
                style=(format!("width: {:.1}%", share * 100.0))
                aria-hidden="true"
            ></span>
            <span class=(if monospace { "relative truncate font-mono text-[13px]" } else { "relative truncate" })>(label)</span>
            <span class="relative shrink-0 tabular-nums text-muted-foreground">(value)</span>
        </li>
    })
}

/// The views and visitors of each day, as bars side by side, with the numbers in a
/// table under them.
#[component]
async fn daily_chart(report: &Report) -> Result<impl View> {
    let peak = report.days.iter().map(|day| day.views).max().unwrap_or(0);
    let top = nice_ceiling(peak);
    let height = move |count: usize| format!("height: {:.1}%", share(count, top) * 100.0);
    let last = report.days.len().saturating_sub(1);
    // The first, middle and last days name the axis.
    let labeled = move |index: usize| index == 0 || index == last || index == last / 2;

    Ok(view! {
        <section class="flex flex-col gap-4 rounded-xl border border-border bg-card p-5">
            <div class="flex flex-wrap items-center justify-between gap-3">
                <h2 class="font-semibold">"Each day"</h2>
                <div class="flex items-center gap-4 text-xs text-muted-foreground">
                    <span class="flex items-center gap-1.5"><span class="size-2.5 rounded-sm bg-chart-1"></span>"Page views"</span>
                    <span class="flex items-center gap-1.5"><span class="size-2.5 rounded-sm bg-chart-2"></span>"Visitors"</span>
                </div>
            </div>
            <div class="flex gap-3">
                <div class="flex h-48 flex-col justify-between text-right text-xs text-muted-foreground tabular-nums">
                    <span class="-translate-y-1/2">(grouped(top))</span>
                    <span class="-translate-y-1/2">(grouped(top / 2))</span>
                    <span class="translate-y-1/2">"0"</span>
                </div>
                <div class="flex min-w-0 flex-1 flex-col gap-2">
                    <div class="relative h-48">
                        <div class="absolute inset-x-0 top-0 border-t border-border/70"></div>
                        <div class="absolute inset-x-0 top-1/2 border-t border-border/70"></div>
                        <div class="absolute inset-0 flex items-end gap-0.5 border-b border-border">
                            #[key(day.date.to_string())]
                            for day in &report.days {
                                <div
                                    class="group flex h-full flex-1 items-end justify-center gap-px rounded-t hover:bg-foreground/5"
                                    title=(format!("{}: {} page views, {} visitors", long_date(day.date), day.views, day.visitors))
                                >
                                    <span class="w-1/2 max-w-3 rounded-t bg-chart-1" style=(height(day.views))></span>
                                    <span class="w-1/2 max-w-3 rounded-t bg-chart-2" style=(height(day.visitors))></span>
                                </div>
                            }
                        </div>
                    </div>
                    <div class="flex text-xs text-muted-foreground">
                        #[key(index)]
                        for (index, day) in report.days.iter().enumerate() {
                            <span class="flex-1 text-center whitespace-nowrap">
                                if labeled(index) { (short_date(day.date)) }
                            </span>
                        }
                    </div>
                </div>
            </div>
            <details class="text-sm">
                <summary class="cursor-pointer text-muted-foreground hover:text-foreground">"Show the numbers"</summary>
                <table class="mt-3 w-full text-left tabular-nums">
                    <thead class="text-xs text-muted-foreground">
                        <tr><th class="py-1.5 font-medium">"Day"</th><th class="py-1.5 text-right font-medium">"Page views"</th><th class="py-1.5 text-right font-medium">"Visitors"</th></tr>
                    </thead>
                    <tbody class="divide-y divide-border">
                        #[key(day.date.to_string())]
                        for day in report.days.iter().rev() {
                            <tr><td class="py-1.5">(long_date(day.date))</td><td class="py-1.5 text-right">(grouped(day.views))</td><td class="py-1.5 text-right">(grouped(day.visitors))</td></tr>
                        }
                    </tbody>
                </table>
            </details>
        </section>
    })
}

/// The admin's name and password, as the form sends them.
#[derive(Deserialize)]
struct SignInForm {
    name: String,
    password: String,
}

/// Signs the admin in, or shows the form again saying it was wrong.
#[route(POST "./sign-in")]
async fn sign_in(cx: &Cx, Form(form): Form<SignInForm>) -> Result<SeeOther> {
    let signed = admin::sign_in(cx, &form.name, &form.password).await?;
    Ok(see_other(if signed { "/admin" } else { "/admin?wrong=1" }))
}

/// Signs the admin out.
#[route(POST "./sign-out")]
async fn sign_out(cx: &Cx) -> Result<SeeOther> {
    admin::sign_out(cx).await?;
    Ok(see_other("/admin"))
}

/// `count` as a share of `of`, from 0 to 1.
fn share(count: usize, of: usize) -> f64 {
    if of == 0 { 0.0 } else { count as f64 / of as f64 }
}

/// `count` as a percentage of `of`, such as `40%`, or a dash when `of` is 0.
fn percent(count: usize, of: usize) -> String {
    if of == 0 {
        "–".to_owned()
    } else {
        format!("{:.0}%", share(count, of) * 100.0)
    }
}

/// The smallest of 1, 2 or 5 times a power of ten that is at least `count`, and whose
/// half is a whole number too, so the axis and its middle read in round numbers.
fn nice_ceiling(count: usize) -> usize {
    let mut power = 1;
    loop {
        for step in [1, 2, 5] {
            let top = step * power;
            if top >= count.max(1) && top % 2 == 0 {
                return top;
            }
        }
        power *= 10;
    }
}

/// A change with its sign, such as `+12`, `0` or `-3`.
fn signed(change: i64) -> String {
    let size = grouped(change.unsigned_abs() as usize);
    match change.signum() {
        1 => format!("+{size}"),
        -1 => format!("-{size}"),
        _ => size,
    }
}

/// How long ago `at` was, such as `5 minutes ago`.
fn ago(at: Timestamp) -> String {
    let seconds = (Timestamp::now().as_second() - at.as_second()).max(0);
    let (count, unit) = match seconds {
        0..60 => return "just now".to_owned(),
        60..3600 => (seconds / 60, "minute"),
        3600..86400 => (seconds / 3600, "hour"),
        _ => (seconds / 86400, "day"),
    };
    format!("{count} {unit}{} ago", if count == 1 { "" } else { "s" })
}

/// `count` with its thousands apart, such as `12,345`.
fn grouped(count: usize) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// A day as `Nov 4`.
fn short_date(date: Date) -> String {
    format!("{} {}", MONTHS[(date.month() - 1) as usize], date.day())
}

/// A day as `Wed, Nov 4`.
fn long_date(date: Date) -> String {
    let weekday = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][date.weekday().to_monday_zero_offset() as usize];
    format!("{weekday}, {}", short_date(date))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_read_in_round_numbers() {
        assert_eq!(nice_ceiling(0), 2);
        assert_eq!(nice_ceiling(1), 2);
        assert_eq!(nice_ceiling(3), 10);
        assert_eq!(nice_ceiling(7), 10);
        assert_eq!(nice_ceiling(40), 50);
        assert_eq!(nice_ceiling(130), 200);
        assert_eq!(nice_ceiling(1_000), 1_000);
    }

    #[test]
    fn changes_carry_their_sign() {
        assert_eq!(signed(12), "+12");
        assert_eq!(signed(0), "0");
        assert_eq!(signed(-1234), "-1,234");
    }

    #[test]
    fn numbers_group_their_thousands() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(12_345), "12,345");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }
}
