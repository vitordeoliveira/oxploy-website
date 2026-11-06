//! The documentation: pages written in Markdown in `docs/`, grouped in sections for the
//! sidebar, and rendered to HTML once, the first time they are asked for.
//!
//! To add a page, write `docs/<slug>.md`, without a `#` title, and list it in
//! [`SECTIONS`] with its title, a sentence about it, and what search results show.

use std::collections::HashMap;
use std::sync::LazyLock;

use pulldown_cmark::{CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};

/// A page of the documentation.
pub struct Page {
    /// Its address under `/docs/`, and its file's name in `docs/`.
    pub slug: &'static str,
    /// Its name in the sidebar, and its heading.
    pub title: &'static str,
    /// A sentence about it, shown under its title.
    pub summary: &'static str,
    /// Its title in search results and link previews, which says what someone would
    /// search for. With ` · oxploy docs` after it, it fits in 60 characters.
    pub search_title: &'static str,
    /// What search results and link previews show under its title: what the page
    /// covers, in 70 to 160 characters.
    pub description: &'static str,
    markdown: &'static str,
}

/// A group of pages in the sidebar.
pub struct Section {
    pub title: &'static str,
    pub pages: &'static [Page],
}

/// A page, with its Markdown from `docs/<slug>.md`.
macro_rules! page {
    (
        slug: $slug:literal,
        title: $title:literal,
        summary: $summary:literal,
        search_title: $search_title:literal,
        description: $description:literal $(,)?
    ) => {
        Page {
            slug: $slug,
            title: $title,
            summary: $summary,
            search_title: $search_title,
            description: $description,
            markdown: include_str!(concat!("../../docs/", $slug, ".md")),
        }
    };
}

/// The documentation, in the order the sidebar shows it.
pub static SECTIONS: &[Section] = &[
    Section {
        title: "Getting started",
        pages: &[
            page! {
                slug: "introduction",
                title: "Introduction",
                summary: "What oxploy is, and how it runs your apps.",
                search_title: "What oxploy is, and how it deploys Rust apps",
                description: "oxploy is a self-hosted deploy panel for Rust: it builds apps from GitHub with cargo, runs them as processes and serves them through Caddy with HTTPS.",
            },
            page! {
                slug: "installation",
                title: "Installation",
                summary: "Install oxploy and Caddy on a Linux server, and set it up.",
                search_title: "Install oxploy and Caddy on a Linux server",
                description: "Install oxploy on Ubuntu or Debian: prepare git and Rust, install Caddy, run oxploy as a systemd service and open its panel over HTTPS with the setup code.",
            },
            page! {
                slug: "github",
                title: "Connecting GitHub",
                summary: "Let oxploy read the repositories it deploys.",
                search_title: "Connect GitHub and deploy private repositories",
                description: "oxploy registers a private GitHub App on your account or organization that can only read code, and deploys the repositories you choose, private ones too.",
            },
        ],
    },
    Section {
        title: "Apps",
        pages: &[
            page! {
                slug: "deploying",
                title: "Deploying",
                summary: "Turn a repository into a running app, and keep it running.",
                search_title: "Deploy a Rust app from GitHub with cargo",
                description: "Deploy a Rust service: pick its repository and branch, the Rust version, binary and features, set its environment, run migrations, and keep it running.",
            },
            page! {
                slug: "domains",
                title: "Domains",
                summary: "Serve apps at your own domains over HTTPS.",
                search_title: "Custom domains with HTTPS through Caddy",
                description: "Serve Rust apps at your own domains with Let's Encrypt certificates from Caddy, with paths, www redirects, and an sslip.io address for every new app.",
            },
            page! {
                slug: "storage",
                title: "Storage",
                summary: "Keep files and SQLite databases across deploys, and back them up.",
                search_title: "Storage folders, SQLite databases and backups",
                description: "Keep an app's files across deploys in its storage folder, connect SQLite databases to it through a variable or a link, and back them up and restore them.",
            },
            page! {
                slug: "monitoring",
                title: "Monitoring",
                summary: "See the CPU, memory and disk your apps take.",
                search_title: "Monitor the CPU, memory and disk of your apps",
                description: "oxploy samples the CPU, memory and disk of each app, of the server and of itself every two seconds, with nothing to install, and charts the last ten minutes.",
            },
        ],
    },
    Section {
        title: "Server",
        pages: &[
            page! {
                slug: "updating",
                title: "Updating",
                summary: "Update oxploy from the panel, and go back if you need to.",
                search_title: "Update oxploy, and go back if you need to",
                description: "oxploy looks for a new release every hour. Update it from the panel while your apps keep running, go back to the previous version, or update it by hand.",
            },
            page! {
                slug: "configuration",
                title: "Configuration",
                summary: "The environment variables oxploy and your apps read.",
                search_title: "Environment variables of oxploy and its apps",
                description: "The environment variables oxploy reads when it starts, such as DATABASE_URL and OXPLOY_DATA_DIR, and those each app gets, such as PORT and OXPLOY_STORAGE_DIR.",
            },
        ],
    },
];

/// Every page, in order.
pub fn pages() -> impl Iterator<Item = &'static Page> {
    SECTIONS.iter().flat_map(|section| section.pages)
}

/// The page at `slug`, if there is one.
pub fn find(slug: &str) -> Option<&'static Page> {
    pages().find(|page| page.slug == slug)
}

/// The page the documentation starts at.
pub fn first() -> &'static Page {
    &SECTIONS[0].pages[0]
}

/// A heading of a page, for its table of contents.
pub struct Heading {
    /// The anchor it has on the page.
    pub id: String,
    pub text: String,
    /// Whether it is under another heading, as a `###` is.
    pub nested: bool,
}

/// A page as HTML.
pub struct Rendered {
    pub html: String,
    pub headings: Vec<Heading>,
}

/// The pages as HTML, rendered once.
static RENDERED: LazyLock<HashMap<&'static str, Rendered>> =
    LazyLock::new(|| pages().map(|page| (page.slug, render(page.markdown))).collect());

impl Page {
    /// The section the page is in.
    pub fn section(&self) -> &'static Section {
        SECTIONS
            .iter()
            .find(|section| section.pages.iter().any(|page| page.slug == self.slug))
            .expect("every page is in a section")
    }

    /// The page as HTML.
    pub fn rendered(&self) -> &'static Rendered {
        &RENDERED[self.slug]
    }

    /// The pages before and after this one, for reading on.
    pub fn neighbors(&self) -> (Option<&'static Page>, Option<&'static Page>) {
        let all: Vec<&'static Page> = pages().collect();
        let index = all.iter().position(|page| page.slug == self.slug).unwrap_or(0);
        let previous = index.checked_sub(1).map(|index| all[index]);
        (previous, all.get(index + 1).copied())
    }
}

/// Renders `markdown` to HTML, giving its `##` and `###` headings anchors.
fn render(markdown: &str) -> Rendered {
    let options = Options::ENABLE_TABLES | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut events: Vec<Event> = Parser::new_ext(markdown, options).collect();
    let mut headings = Vec::new();
    let mut taken = HashMap::<String, usize>::new();
    for start in 0..events.len() {
        let Event::Start(Tag::Heading { level, id, .. }) = &events[start] else {
            continue;
        };
        if !matches!(level, HeadingLevel::H2 | HeadingLevel::H3) {
            continue;
        }
        let nested = *level == HeadingLevel::H3;
        let mut text = String::new();
        for event in &events[start + 1..] {
            match event {
                Event::Text(part) | Event::Code(part) => text.push_str(part),
                Event::End(TagEnd::Heading(_)) => break,
                _ => {}
            }
        }
        let id = match id {
            Some(id) => id.to_string(),
            None => {
                let base = anchor(&text);
                let count = taken.entry(base.clone()).or_default();
                *count += 1;
                if *count == 1 { base } else { format!("{base}-{count}") }
            }
        };
        if let Event::Start(Tag::Heading { id: slot, .. }) = &mut events[start] {
            *slot = Some(CowStr::from(id.clone()));
        }
        headings.push(Heading { id, text, nested });
    }
    let mut out = String::new();
    html::push_html(&mut out, events.into_iter());
    Rendered { html: out, headings }
}

/// An anchor for a heading's text, such as `run-it-as-a-service`.
fn anchor(text: &str) -> String {
    let mut anchor = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            anchor.extend(c.to_lowercase());
        } else if !anchor.ends_with('-') && !anchor.is_empty() {
            anchor.push('-');
        }
    }
    anchor.trim_end_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_get_anchors() {
        let rendered = render("## Run it as a service\n\ntext\n\n### `PORT`\n\n## Run it as a service\n\n## Custom {#own}\n");
        let ids: Vec<&str> = rendered.headings.iter().map(|heading| heading.id.as_str()).collect();
        assert_eq!(ids, ["run-it-as-a-service", "port", "run-it-as-a-service-2", "own"]);
        assert!(rendered.headings[1].nested);
        assert!(rendered.html.contains(r#"<h2 id="run-it-as-a-service">"#));
    }

    #[test]
    fn every_page_fits_in_a_search_result() {
        for page in pages() {
            let title = format!("{} · oxploy docs", page.search_title);
            assert!(title.chars().count() <= 60, "{}'s search title is too long", page.slug);
            let description = page.description.chars().count();
            assert!((70..=160).contains(&description), "{}'s description has {description} characters", page.slug);
            assert_eq!(pages().filter(|other| other.description == page.description).count(), 1, "{} shares its description", page.slug);
        }
    }

    #[test]
    fn every_page_renders() {
        for page in pages() {
            assert!(!page.rendered().html.is_empty(), "{} is empty", page.slug);
            assert!(!page.markdown.trim_start().starts_with("# "), "{} has its own title", page.slug);
        }
    }
}
