pub mod admin;
pub mod docs;
pub mod privacy;

use serde_json::{Value, json};
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt, asset, asset_config},
    context::Cx,
    cookie::RouterBuilderCookieExt,
    font::{Font, fontsource::fontsource_font},
    icon::{IconData, icon, iconify::iconify_icon},
    router::{
        RouterBuilder, RouterBuilderDiscoverExt, Slot, content::sitemap::Sitemap, layout, module_router, page,
        request::uri, route,
    },
    runtime::RouterBuilderRuntimeExt,
    session::{RouterBuilderSessionExt, SessionConfig},
    tailwind,
    view::{Unescaped, View, attributes, component, view},
};

use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::features::analytics::{self, Analytics};
use crate::features::docs as guide;
use crate::features::seo;

/// oxploy's repository, where its code and releases are.
pub const REPOSITORY: &str = "https://github.com/vitordeoliveira/oxploy";

/// The command that downloads the latest release, as the installation guide runs it.
const DOWNLOAD: &str = "curl -L https://github.com/vitordeoliveira/oxploy/releases/latest/download/oxploy-x86_64-linux.tar.gz | tar xz -C /opt/oxploy";

/// The oxploy logo, a cog like the one in the Rust logo, as in the panel.
const LOGO: IconData = iconify_icon!("lucide:cog");

/// The Inter font family, loaded from Fontsource, as in the panel.
const INTER: Font = fontsource_font!(INTER, weight: [400, 500, 600, 700], style: Normal);

/// What the site says about oxploy in a sentence, for search engines and link previews.
const DESCRIPTION: &str = "oxploy is a self-hosted deploy panel for Rust: it builds your apps from GitHub with cargo and runs them on your own server behind Caddy, with HTTPS. No Docker.";

/// What the privacy page says it is about, for search engines.
const PRIVACY_DESCRIPTION: &str = "What the oxploy site keeps about its visitors: a count of visits made on its own server, and a cookie only for those who accept it.";

/// What the picture link previews show is, for those who cannot see it.
const PREVIEW_ALT: &str = "oxploy: deploy Rust apps on your own server. Builds with cargo, HTTPS through Caddy, no Docker.";

/// Builds the site's router.
pub fn router() -> RouterBuilder {
    // The pages and layouts of this module and its submodules, with paths derived
    // from the module tree.
    module_router!()
        // The runtime's procedures and shards, which have no place in the module tree.
        .discover()
        .assets(AssetBundle::load().unwrap())
        // The admin's session, carried in a cookie.
        .cookies()
        .sessions(SessionConfig::default())
        // Lets the browser runtime rerun pages; register it after other layers.
        .runtime()
}

/// The address of the documentation's page at `slug`.
fn doc(slug: &str) -> String {
    format!("/docs/{slug}")
}

/// The site's pages that search engines should find, as the sitemap lists them. A new
/// page goes here too.
fn listed_pages() -> impl Iterator<Item = String> {
    ["/".to_owned(), "/privacy".to_owned()]
        .into_iter()
        .chain(guide::pages().map(|page| doc(page.slug)))
}

/// The sitemap, which gives search engines the address of every page.
#[route(GET "./sitemap.xml")]
async fn sitemap() -> Result<Sitemap> {
    Ok(Sitemap::new().urls(listed_pages().map(|path| seo::url(&path))))
}

/// What crawlers may read, and where the sitemap is.
#[route(GET "./robots.txt")]
async fn robots() -> Result<String> {
    Ok(seo::robots())
}

#[layout]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let path = uri(cx).path();
    let admin = path == "/admin" || path.starts_with("/admin/");
    // This layout also runs for a missing page or a redirect, so only the site's pages
    // are given to search engines, and count as visits but for the privacy page.
    let listed = listed_pages().any(|listed| listed == path);
    if listed && path != "/privacy" {
        Analytics::get(cx).record(cx);
    }
    // A documentation page is named after what someone would search for.
    let page = path.strip_prefix("/docs/").and_then(guide::find);
    let title = match page {
        Some(page) => format!("{} · oxploy docs", page.search_title),
        None if admin => "Admin · oxploy".to_owned(),
        None if path == "/privacy" => "Privacy · oxploy".to_owned(),
        None => "oxploy · Deploy Rust apps on your own server".to_owned(),
    };
    let description = match page {
        Some(page) => page.description,
        None if path == "/privacy" => PRIVACY_DESCRIPTION,
        None => DESCRIPTION,
    };
    // The one address of the page, without the query, such as a link's utm_source, so
    // search engines do not count the same page twice.
    let address = seo::url(path);
    let preview = seo::url(&asset_config(cx).resolve(asset!("assets/preview.jpg")));
    // The privacy page asks the same itself, and the admin is not a visitor.
    let ask = !admin && path != "/privacy" && analytics::choice(cx).is_none();

    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>(&title)</title>
                <meta name="description" content=(description)>
                if listed {
                    <link rel="canonical" href=(&address)>
                } else {
                    <meta name="robots" content="noindex">
                }
                <meta property="og:type" content=(if page.is_some() { "article" } else { "website" })>
                <meta property="og:site_name" content=(seo::SITE_NAME)>
                <meta property="og:locale" content="en_US">
                <meta property="og:url" content=(&address)>
                <meta property="og:title" content=(&title)>
                <meta property="og:description" content=(description)>
                <meta property="og:image" content=(&preview)>
                <meta property="og:image:width" content="1200">
                <meta property="og:image:height" content="630">
                <meta property="og:image:alt" content=(PREVIEW_ALT)>
                <meta name="twitter:card" content="summary_large_image">
                // The page's background, from `styles.css`, for the browser's bar.
                <meta name="theme-color" media="(prefers-color-scheme: light)" content="#fefdfc">
                <meta name="theme-color" media="(prefers-color-scheme: dark)" content="#120f0d">
                <link rel="icon" type="image/svg+xml" href=(asset!("assets/favicon.svg"))>
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
                topcoat::font::link(font: INTER)
                topcoat::runtime::script()
                topcoat::dev::script()
            </head>
            <body class="flex min-h-svh flex-col">
                site_header()
                <div class="flex flex-1 flex-col">(slot)</div>
                site_footer()
                if ask {
                    privacy::cookie_banner()
                }
            </body>
        </html>
    })
}

/// oxploy's name beside its logo, linking home.
#[component]
async fn brand() -> Result<impl View> {
    Ok(view! {
        <a href="/" class="flex items-center gap-2 font-semibold tracking-tight">
            <span class="flex size-7 items-center justify-center rounded-md bg-primary text-primary-foreground">
                icon(data: LOGO, size: 18)
            </span>
            "oxploy"
        </a>
    })
}

/// The bar at the top of every page.
#[component]
async fn site_header() -> Result<impl View> {
    Ok(view! {
        <header class="sticky top-0 z-40 border-b border-border/70 bg-background/85 backdrop-blur">
            <nav class="mx-auto flex h-16 max-w-6xl items-center gap-4 px-4 sm:gap-6 sm:px-6" aria-label="Main">
                brand()
                <div class="flex items-center text-sm">
                    <a href="/#features" class="hidden rounded-md px-3 py-2 text-muted-foreground hover:text-foreground sm:block">"Features"</a>
                    <a href=(doc(guide::first().slug)) class="rounded-md px-3 py-2 text-muted-foreground hover:text-foreground">"Docs"</a>
                    <a href=(format!("{REPOSITORY}/releases")) class="hidden rounded-md px-3 py-2 text-muted-foreground hover:text-foreground sm:block">"Releases"</a>
                </div>
                <div class="ml-auto flex items-center gap-2">
                    <a
                        href=(REPOSITORY)
                        aria-label="oxploy on GitHub"
                        title="oxploy on GitHub"
                        class=(button_variants(ButtonVariant::Ghost, ButtonSize::Icon))
                    >
                        icon(data: iconify_icon!("lucide:github"), size: 18)
                    </a>
                    <a href=(doc("installation")) class=(button_variants(ButtonVariant::Primary, ButtonSize::Sm))>
                        "Get started"
                    </a>
                </div>
            </nav>
        </header>
    })
}

/// The links at the foot of every page.
#[component]
async fn site_footer() -> Result<impl View> {
    let product = [
        ("Features", "/#features".to_owned()),
        ("How it works", "/#how-it-works".to_owned()),
        ("FAQ", "/#faq".to_owned()),
    ];
    let guides = [
        ("Introduction", doc("introduction")),
        ("Installation", doc("installation")),
        ("Deploying", doc("deploying")),
        ("Domains", doc("domains")),
    ];
    let project = [
        ("GitHub", REPOSITORY.to_owned()),
        ("Releases", format!("{REPOSITORY}/releases")),
        ("Issues", format!("{REPOSITORY}/issues")),
        ("Privacy", "/privacy".to_owned()),
    ];

    Ok(view! {
        <footer class="border-t border-border">
            <div class="mx-auto grid max-w-6xl gap-10 px-4 py-12 sm:grid-cols-3 sm:px-6 lg:grid-cols-[2fr_1fr_1fr_1fr]">
                <div class="flex flex-col gap-3 sm:col-span-3 lg:col-span-1">
                    brand()
                    <p class="max-w-xs text-sm text-muted-foreground">
                        "Deploy Rust apps on your own server."
                    </p>
                </div>
                footer_links(title: "Product", links: &product)
                footer_links(title: "Docs", links: &guides)
                footer_links(title: "Project", links: &project)
            </div>
        </footer>
    })
}

/// A column of the footer's links.
#[component]
async fn footer_links(title: &str, links: &[(&str, String)]) -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-col gap-3 text-sm">
            <p class="font-medium">(title)</p>
            <ul class="flex flex-col gap-2">
                #[key(url.clone())]
                for (label, url) in links {
                    <li><a href=(url) class="text-muted-foreground hover:text-foreground">(*label)</a></li>
                }
            </ul>
        </div>
    })
}

/// The questions the home page answers, with their answers.
const FAQ: [(&str, &str); 6] = [
    (
        "Does it need Docker?",
        "No. oxploy builds apps with cargo on the server and runs each one as a process of its own, listening on 127.0.0.1. Caddy, which runs beside it, sends each domain to its app.",
    ),
    (
        "Which apps can it deploy?",
        "Rust apps that cargo builds into a binary that listens on the port in PORT. A build script of your own can produce that binary any other way.",
    ),
    (
        "What does the server need?",
        "Linux on x86_64, git, Rust installed with rustup, a C compiler, and Caddy. oxploy itself is a binary with a folder of assets beside it.",
    ),
    (
        "Can one server run many apps?",
        "Yes. Each app gets a port of its own and as many domains as you give it, and the monitoring shows what each one takes.",
    ),
    (
        "What happens to my apps when oxploy restarts or updates?",
        "They keep running. oxploy writes down each app's process, and takes it back when it starts again.",
    ),
    (
        "Where is my data?",
        "On your server. Each app has a storage folder that deploys keep, and oxploy's own database is a SQLite file.",
    ),
];

#[page]
async fn home() -> Result<impl View> {
    let installation = doc("installation");
    let home = seo::url("/");
    // The site, oxploy as an app, and the questions below, for search engines.
    let data = json!({
        "@context": "https://schema.org",
        "@graph": [
            {
                "@type": "WebSite",
                "name": seo::SITE_NAME,
                "url": home,
                "description": DESCRIPTION,
                "inLanguage": "en",
            },
            {
                "@type": "SoftwareApplication",
                "name": "oxploy",
                "description": DESCRIPTION,
                "url": home,
                "applicationCategory": "DeveloperApplication",
                "operatingSystem": "Linux",
                "downloadUrl": format!("{REPOSITORY}/releases/latest"),
                "installUrl": seo::url(&installation),
                "sameAs": [REPOSITORY],
            },
            {
                "@type": "FAQPage",
                "mainEntity": FAQ.iter().map(|(asked, answer)| json!({
                    "@type": "Question",
                    "name": asked,
                    "acceptedAnswer": { "@type": "Answer", "text": answer },
                })).collect::<Vec<_>>(),
            },
        ],
    });

    Ok(view! {
        structured_data(data: data)
        <main>
            // The opening: what oxploy is, and how to start.
            <section class="relative overflow-hidden border-b border-border">
                <div class="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_top,var(--color-primary)_0%,transparent_60%)] opacity-[0.12]"></div>
                <div class="relative mx-auto grid max-w-6xl items-center gap-12 px-4 py-20 sm:px-6 lg:grid-cols-[1.1fr_1fr] lg:py-28">
                    <div class="flex min-w-0 flex-col items-start gap-6">
                        <span class="inline-flex items-center gap-2 rounded-full border border-border bg-card px-3 py-1 text-xs font-medium text-muted-foreground">
                            <span class="size-1.5 rounded-full bg-primary"></span>
                            "Self-hosted, built for Rust"
                        </span>
                        <h1 class="text-4xl font-semibold tracking-tight text-balance sm:text-5xl lg:text-6xl">
                            "Deploy Rust apps on " <span class="text-primary">"your own server"</span>
                        </h1>
                        <p class="max-w-xl text-lg text-pretty text-muted-foreground">
                            "oxploy builds your services from GitHub with cargo, runs them as plain processes and serves them through Caddy with HTTPS. One binary to install. No Docker, no YAML."
                        </p>
                        <div class="flex flex-wrap gap-3">
                            <a href=(&installation) class=(button_variants(ButtonVariant::Primary, ButtonSize::Lg))>
                                "Get started"
                                icon(data: iconify_icon!("lucide:arrow-right"), size: 16)
                            </a>
                            <a href=(REPOSITORY) class=(button_variants(ButtonVariant::Outline, ButtonSize::Lg))>
                                icon(data: iconify_icon!("lucide:github"), size: 16)
                                "View on GitHub"
                            </a>
                        </div>
                        <div class="w-full max-w-xl">
                            <p class="mb-2 text-xs font-medium text-muted-foreground">"Download the latest release on a Linux server"</p>
                            <pre class="rounded-lg border border-border bg-card px-4 py-3 font-mono text-xs leading-6 break-all whitespace-pre-wrap"><code><span class="text-muted-foreground select-none">"$ "</span>(DOWNLOAD)</code></pre>
                        </div>
                    </div>
                    deploy_log()
                </div>
            </section>

            <section id="features" class="scroll-mt-16 border-b border-border">
                <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6">
                    section_heading(
                        eyebrow: "Features",
                        title: "Everything a Rust service needs to run",
                        text: "From the push to a live address, on a machine you control. oxploy takes care of the build, the process, the certificate and the data."
                    )
                    <div class="mt-12 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                        feature(
                            symbol: iconify_icon!("lucide:hammer"),
                            title: "Builds with cargo",
                            text: "Pick the Rust version, the binary and the features, or write your own build script. Workspaces build from any folder."
                        )
                        feature(
                            symbol: iconify_icon!("lucide:git-branch"),
                            title: "Deploys from GitHub",
                            text: "Connect an account through a private GitHub App that can only read code. Private repositories work."
                        )
                        feature(
                            symbol: iconify_icon!("lucide:cpu"),
                            title: "Processes, not containers",
                            text: "Each app runs as a process on 127.0.0.1, in a group of its own. Apps keep running while oxploy restarts."
                        )
                        feature(
                            symbol: iconify_icon!("lucide:lock"),
                            title: "Domains with HTTPS",
                            text: "Caddy serves every domain with a Let's Encrypt certificate, with path prefixes and www redirects. New apps get an address of their own."
                        )
                        feature(
                            symbol: iconify_icon!("lucide:database"),
                            title: "SQLite and backups",
                            text: "A storage folder that deploys keep, SQLite databases connected to the app, and backups you restore in a click."
                        )
                        feature(
                            symbol: iconify_icon!("lucide:activity"),
                            title: "Monitoring",
                            text: "CPU, memory and disk for each app, the machine and oxploy itself, sampled every two seconds."
                        )
                        feature(
                            symbol: iconify_icon!("lucide:shield-check"),
                            title: "Safe releases",
                            text: "A script after the build, such as a migration, runs before the new release starts. If it fails, the running release stays."
                        )
                        feature(
                            symbol: iconify_icon!("lucide:refresh-cw"),
                            title: "Restarts on crashes",
                            text: "If an app exits, oxploy starts it again, waiting longer after each crash, and keeps its log."
                        )
                        feature(
                            symbol: iconify_icon!("lucide:download"),
                            title: "Updates itself",
                            text: "When a new release is out, the panel says so. One click installs it, and the apps keep running."
                        )
                    </div>
                </div>
            </section>

            <section id="how-it-works" class="scroll-mt-16 border-b border-border bg-muted/40">
                <div class="mx-auto max-w-6xl px-4 py-20 sm:px-6">
                    section_heading(
                        eyebrow: "How it works",
                        title: "From a fresh server to a live app",
                        text: "oxploy is a single binary that runs beside Caddy. The panel, the builds and the apps all live on the same machine."
                    )
                    <ol class="mt-12 grid gap-4 md:grid-cols-3">
                        step(
                            number: 1,
                            title: "Install it",
                            text: "Download the release, run it as a systemd service and install Caddy. The panel opens over HTTPS at the server's address, and a setup code from its log makes you its owner."
                        )
                        step(
                            number: 2,
                            title: "Connect GitHub",
                            text: "From the panel, oxploy registers a GitHub App on your account or organization. Install it on the repositories you want to deploy."
                        )
                        step(
                            number: 3,
                            title: "Deploy",
                            text: "Create a service, choose its repository and branch, and deploy. oxploy builds it, starts it on a port of its own and gives it a domain."
                        )
                    </ol>
                    request_path()
                </div>
            </section>

            <section id="faq" class="scroll-mt-16 border-b border-border">
                <div class="mx-auto grid max-w-6xl gap-12 px-4 py-20 sm:px-6 lg:grid-cols-[1fr_2fr]">
                    section_heading(
                        eyebrow: "FAQ",
                        title: "Frequently asked questions",
                        text: "Something else? Ask in the repository's issues."
                    )
                    <div class="divide-y divide-border border-y border-border">
                        #[key(*asked)]
                        for (asked, answer) in FAQ {
                            question(asked: asked, answer: answer)
                        }
                    </div>
                </div>
            </section>

            <section class="mx-auto flex max-w-6xl flex-col items-center gap-6 px-4 py-24 text-center sm:px-6">
                <h2 class="text-3xl font-semibold tracking-tight text-balance sm:text-4xl">
                    "Ship your next Rust service on your own server"
                </h2>
                <p class="max-w-xl text-muted-foreground">
                    "Install oxploy in a few minutes, and deploy from the panel from then on."
                </p>
                <div class="flex flex-wrap justify-center gap-3">
                    <a href=(&installation) class=(button_variants(ButtonVariant::Primary, ButtonSize::Lg))>
                        "Read the installation guide"
                    </a>
                    <a href=(doc(guide::first().slug)) class=(button_variants(ButtonVariant::Outline, ButtonSize::Lg))>
                        "Browse the docs"
                    </a>
                </div>
            </section>
        </main>
    })
}

/// A deployment's log, as the panel shows it, beside the opening.
#[component]
async fn deploy_log() -> Result<impl View> {
    // The lines oxploy writes for a service named shop-api, and what cargo and the
    // script after the build print between them.
    let lines = [
        (true, "==> Fetching acme/shop-api at main"),
        (true, "==> Building with cargo build --release --bin shop-api"),
        (false, "   Compiling shop-api v0.4.2"),
        (false, "    Finished `release` profile [optimized] target(s) in 41.30s"),
        (true, "==> Running the script after the build"),
        (false, "applied 2 migrations"),
        (false, "Stopped release #11"),
        (true, "==> Starting on 127.0.0.1:10000"),
        (false, "Listening on 127.0.0.1:10000"),
        (true, "==> Deployed"),
    ];

    Ok(view! {
        <figure class="min-w-0 overflow-hidden rounded-xl border border-border bg-card shadow-sm">
            <div class="flex items-center gap-3 border-b border-border px-4 py-3">
                <span class="flex gap-1.5" aria-hidden="true">
                    <span class="size-2.5 rounded-full bg-border"></span>
                    <span class="size-2.5 rounded-full bg-border"></span>
                    <span class="size-2.5 rounded-full bg-border"></span>
                </span>
                <span class="text-xs font-medium">"Deployment #12"</span>
                <span class="ml-auto inline-flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-2 py-0.5 text-[11px] font-medium text-emerald-700 dark:text-emerald-400">
                    <span class="size-1.5 rounded-full bg-emerald-500"></span>
                    "Live"
                </span>
            </div>
            <pre class="overflow-x-auto px-4 py-4 font-mono text-[12px] leading-6"><code>
                #[key(index)]
                for (index, (is_step, line)) in lines.iter().enumerate() {
                    <span class=(if *is_step { "block text-foreground" } else { "block text-muted-foreground" })>(*line)</span>
                }
            </code></pre>
            <figcaption class="flex items-center gap-2 border-t border-border px-4 py-3 text-xs">
                icon(data: iconify_icon!("lucide:lock"), size: 12, attrs: attributes! { class="shrink-0 text-emerald-600 dark:text-emerald-400" })
                <span class="truncate font-mono">"https://shop-api.203-0-113-7.sslip.io"</span>
            </figcaption>
        </figure>
    })
}

/// What a page tells search engines about itself, as JSON-LD, which they read and
/// nobody sees.
#[component]
async fn structured_data(data: Value) -> Result<impl View> {
    let json = seo::json_ld(&data);

    Ok(view! {
        // `json_ld` escapes whatever could close the script.
        <script type="application/ld+json">(Unescaped::new_unchecked(json))</script>
    })
}

/// The heading of one of the home page's sections.
#[component]
async fn section_heading(eyebrow: &str, title: &str, text: &str) -> Result<impl View> {
    Ok(view! {
        <div class="flex max-w-2xl flex-col gap-3">
            <p class="text-sm font-medium text-primary">(eyebrow)</p>
            <h2 class="text-3xl font-semibold tracking-tight text-balance">(title)</h2>
            <p class="text-pretty text-muted-foreground">(text)</p>
        </div>
    })
}

/// One of the features, in the grid.
#[component]
async fn feature(symbol: IconData, title: &str, text: &str) -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-col gap-3 rounded-xl border border-border bg-card p-5">
            <span class="flex size-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
                icon(data: symbol, size: 18)
            </span>
            <h3 class="font-semibold">(title)</h3>
            <p class="text-sm text-muted-foreground">(text)</p>
        </div>
    })
}

/// One of the steps of how it works.
#[component]
async fn step(number: u8, title: &str, text: &str) -> Result<impl View> {
    Ok(view! {
        <li class="flex flex-col gap-3 rounded-xl border border-border bg-card p-5">
            <span class="flex size-8 items-center justify-center rounded-full border border-primary/40 font-mono text-sm font-semibold text-primary">
                (number.to_string())
            </span>
            <h3 class="font-semibold">(title)</h3>
            <p class="text-sm text-muted-foreground">(text)</p>
        </li>
    })
}

/// Where a request goes, from the browser to an app, under the steps.
#[component]
async fn request_path() -> Result<impl View> {
    Ok(view! {
        <div class="mt-10 flex flex-col items-stretch gap-3 rounded-xl border border-dashed border-border p-5 md:flex-row md:items-center">
            hop(symbol: iconify_icon!("lucide:globe"), title: "Browser", detail: "shop.example.com")
            arrow()
            hop(symbol: iconify_icon!("lucide:lock"), title: "Caddy", detail: ":443, with its certificate")
            arrow()
            hop(symbol: iconify_icon!("lucide:box"), title: "Your app", detail: "127.0.0.1:10000")
            <p class="text-sm text-muted-foreground md:ml-4 md:max-w-xs">
                "oxploy gives Caddy every route and runs the apps. The panel answers through Caddy too, at its own domain."
            </p>
        </div>
    })
}

/// A stop on the way to an app.
#[component]
async fn hop(symbol: IconData, title: &str, detail: &str) -> Result<impl View> {
    Ok(view! {
        <div class="flex items-center gap-3 rounded-lg border border-border bg-card px-4 py-3">
            icon(data: symbol, size: 18, attrs: attributes! { class="shrink-0 text-primary" })
            <div class="flex flex-col">
                <span class="text-sm font-medium">(title)</span>
                <span class="font-mono text-xs text-muted-foreground">(detail)</span>
            </div>
        </div>
    })
}

/// The arrow between two stops, down on phones and across on wider screens.
#[component]
async fn arrow() -> Result<impl View> {
    Ok(view! {
        <span class="flex justify-center text-muted-foreground" aria-hidden="true">
            icon(data: iconify_icon!("lucide:arrow-down"), size: 16, attrs: attributes! { class="md:hidden" })
            icon(data: iconify_icon!("lucide:arrow-right"), size: 16, attrs: attributes! { class="hidden md:block" })
        </span>
    })
}

/// A question and its answer, which opens when clicked.
#[component]
async fn question(asked: &str, answer: &str) -> Result<impl View> {
    Ok(view! {
        <details class="group py-4">
            <summary class="flex cursor-pointer list-none items-center justify-between gap-4 font-medium [&::-webkit-details-marker]:hidden">
                (asked)
                icon(data: iconify_icon!("lucide:plus"), size: 16, attrs: attributes! { class="shrink-0 text-muted-foreground transition-transform group-open:rotate-45" })
            </summary>
            <p class="mt-3 text-sm text-muted-foreground">(answer)</p>
        </details>
    })
}
