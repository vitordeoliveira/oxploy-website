//! The documentation: pages written in Markdown in `docs/`, grouped in sections for the
//! sidebar, and rendered to HTML once, the first time they are asked for.
//!
//! To add a page, write `docs/<slug>.md`, without a `#` title, and list it in
//! [`SECTIONS`] with its title and a sentence about it.

use std::collections::HashMap;
use std::sync::LazyLock;

use pulldown_cmark::{CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};

/// A page of the documentation.
pub struct Page {
    /// Its address under `/docs/`, and its file's name in `docs/`.
    pub slug: &'static str,
    pub title: &'static str,
    /// A sentence about it, shown under its title and as its description.
    pub summary: &'static str,
    markdown: &'static str,
}

/// A group of pages in the sidebar.
pub struct Section {
    pub title: &'static str,
    pub pages: &'static [Page],
}

/// A page, with its Markdown from `docs/<slug>.md`.
macro_rules! page {
    ($slug:literal, $title:literal, $summary:literal) => {
        Page {
            slug: $slug,
            title: $title,
            summary: $summary,
            markdown: include_str!(concat!("../../docs/", $slug, ".md")),
        }
    };
}

/// The documentation, in the order the sidebar shows it.
pub static SECTIONS: &[Section] = &[
    Section {
        title: "Getting started",
        pages: &[
            page!("introduction", "Introduction", "What oxploy is, and how it runs your apps."),
            page!("installation", "Installation", "Install oxploy and Caddy on a Linux server, and set it up."),
            page!("github", "Connecting GitHub", "Let oxploy read the repositories it deploys."),
        ],
    },
    Section {
        title: "Apps",
        pages: &[
            page!("deploying", "Deploying", "Turn a repository into a running app, and keep it running."),
            page!("domains", "Domains", "Serve apps at your own domains over HTTPS."),
            page!("storage", "Storage", "Keep files and SQLite databases across deploys, and back them up."),
            page!("monitoring", "Monitoring", "See the CPU, memory and disk your apps take."),
        ],
    },
    Section {
        title: "Server",
        pages: &[
            page!("updating", "Updating", "Update oxploy from the panel, and go back if you need to."),
            page!("configuration", "Configuration", "The environment variables oxploy and your apps read."),
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
    fn every_page_renders() {
        for page in pages() {
            assert!(!page.rendered().html.is_empty(), "{} is empty", page.slug);
            assert!(!page.markdown.trim_start().starts_with("# "), "{} has its own title", page.slug);
        }
    }
}
