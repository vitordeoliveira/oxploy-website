//! The admin's page, at `/admin`: a form to sign in, and once signed in, the site's
//! visits of the last days.

pub mod mcp;

use jiff::civil::Date;
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
    let report = if admin::signed_in(cx).await? {
        Some(Report::of_last(&db(cx), days).await?)
    } else {
        None
    };

    Ok(view! {
        <main class="mx-auto w-full max-w-6xl px-4 py-10 sm:px-6">
            if let Some(report) = &report {
                dashboard(report: report, days: days)
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
async fn dashboard(report: &Report, days: i64) -> Result<impl View> {
    let per_visitor = if report.visitors == 0 {
        "0".to_owned()
    } else {
        format!("{:.1}", report.views as f64 / report.visitors as f64)
    };

    Ok(view! {
        <div class="flex flex-col gap-8">
            <div class="flex flex-wrap items-end justify-between gap-4">
                <div class="flex flex-col gap-1">
                    <h1 class="text-2xl font-semibold tracking-tight">"Visits"</h1>
                    <p class="max-w-2xl text-sm text-muted-foreground">
                        "The people who opened the site's pages, each counted once a day, without keeping their address, whether they accepted the cookie or not. Programs, such as search engines, are left out. Days are in "
                        (&report.time_zone) "."
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

/// A list of how many visitors each value has, such as each browser.
#[component]
async fn tally_card(title: &str, column: &str, rows: &[(String, usize)], empty: &str) -> Result<impl View> {
    Ok(view! {
        <section class="flex flex-col gap-3 rounded-xl border border-border bg-card p-5">
            <h2 class="font-semibold">(title)</h2>
            if rows.is_empty() {
                <p class="text-sm text-muted-foreground">(empty)</p>
            } else {
                <div class="flex justify-between px-3 text-xs text-muted-foreground">
                    <span>(column)</span>
                    <span>"Visitors"</span>
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

/// The smallest of 1, 2 or 5 times a power of ten that is at least `count`, so the
/// axis reads in round numbers.
fn nice_ceiling(count: usize) -> usize {
    let mut power = 1;
    loop {
        for step in [1, 2, 5] {
            if step * power >= count.max(1) {
                return step * power;
            }
        }
        power *= 10;
    }
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
        assert_eq!(nice_ceiling(0), 1);
        assert_eq!(nice_ceiling(3), 5);
        assert_eq!(nice_ceiling(7), 10);
        assert_eq!(nice_ceiling(130), 200);
        assert_eq!(nice_ceiling(1_000), 1_000);
    }

    #[test]
    fn numbers_group_their_thousands() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(12_345), "12,345");
        assert_eq!(grouped(1_234_567), "1,234,567");
    }
}
