//! Searching the documentation: each page is cut into passages at its `##` and `###`
//! headings, and a query finds the passages that have its words, rarest words first.

use topcoat::asset::AssetConfig;

use super::{Heading, Page, absolute_links, pages, pictures};

/// A passage of the documentation that has the words searched for.
pub struct Hit {
    pub page: &'static Page,
    /// The heading the passage starts with, or `None` for the start of the page, before
    /// its first heading.
    pub heading: Option<&'static Heading>,
    /// The passage's Markdown, with its links to the site made absolute.
    pub markdown: String,
}

/// The most words of a query that count. A longer query is cut there.
const MAX_TERMS: usize = 20;

/// A part of a page: its start, before the first heading, or a heading and what follows
/// it until the next one.
struct Passage {
    page: &'static Page,
    heading: Option<&'static Heading>,
    markdown: &'static str,
    /// The words of the passage.
    words: Vec<String>,
    /// The words that say what it is about: the page's title, and its heading.
    titled: Vec<String>,
}

/// The passages of every page, in the order the pages are.
fn passages() -> Vec<Passage> {
    let mut passages = Vec::new();
    for page in pages() {
        let headings = &page.rendered().headings;
        let mut start = 0;
        let mut heading = None;
        for next in headings.iter().map(Some).chain([None]) {
            let end = next.map_or(page.markdown.len(), |next| next.offset);
            let markdown = &page.markdown[start..end];
            if !markdown.trim().is_empty() {
                let about = heading.map_or("", |heading: &Heading| heading.text.as_str());
                passages.push(Passage {
                    page,
                    heading,
                    markdown,
                    words: words(&without_pictures(markdown)).collect(),
                    titled: words(page.title).chain(words(about)).collect(),
                });
            }
            start = end;
            heading = next;
        }
    }
    passages
}

/// The `limit` passages that best match `query`, best first. A passage matches a word
/// that one of its words starts with, so `domain` finds `domains`; a word that few
/// passages have counts more, and more again in the passage's title or heading. A query
/// without a word of two letters or more finds nothing.
pub fn search(query: &str, limit: usize, assets: Option<&AssetConfig>) -> Vec<Hit> {
    let terms = terms(query);
    let passages = passages();
    let count = passages.len() as f64;
    let mut scores = vec![0.0; passages.len()];
    for term in &terms {
        let matches = |words: &[String]| words.iter().filter(|word| word.starts_with(term.as_str())).count();
        let found: Vec<(usize, usize, bool)> = passages
            .iter()
            .enumerate()
            .map(|(index, passage)| (index, matches(&passage.words), matches(&passage.titled) > 0))
            .filter(|(_, times, titled)| *times > 0 || *titled)
            .collect();
        if found.is_empty() {
            continue;
        }
        // Rarer words weigh more, and one in every passage still a little.
        let weight = ((count + 1.0) / found.len() as f64).ln();
        for (index, times, titled) in found {
            scores[index] += weight * (1.0 + (times.max(1) as f64).ln());
            if titled {
                scores[index] += weight;
            }
        }
    }
    let mut ranked: Vec<(usize, f64)> = scores
        .into_iter()
        .enumerate()
        .filter(|(_, score)| *score > 0.0)
        .collect();
    // A stable sort keeps the pages' order between passages that score the same.
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranked
        .into_iter()
        .take(limit)
        .map(|(index, _)| {
            let passage = &passages[index];
            Hit {
                page: passage.page,
                heading: passage.heading,
                markdown: pictures::absolute(&absolute_links(passage.markdown.trim()), assets),
            }
        })
        .collect()
}

/// Whether `query` has a word to search for, of two letters or more.
pub fn has_words(query: &str) -> bool {
    !terms(query).is_empty()
}

/// The words of `query` to search for, each once, without their endings, and up to
/// [`MAX_TERMS`] of them.
fn terms(query: &str) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    for word in words(query).filter(|word| word.chars().count() > 1).map(|word| stem(&word)) {
        if !terms.contains(&word) {
            terms.push(word);
        }
    }
    terms.truncate(MAX_TERMS);
    terms
}

/// `markdown` without its pictures, `![…](…)`, whose descriptions repeat what the text
/// around them says.
fn without_pictures(markdown: &str) -> String {
    let mut text = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(start) = rest.find("![") {
        let Some(end) = rest[start..].find(')') else {
            break;
        };
        text.push_str(&rest[..start]);
        rest = &rest[start + end + 1..];
    }
    text.push_str(rest);
    text
}

/// The words of `text`, in lowercase. An underscore joins a word, so a variable such as
/// `OXPLOY_STORAGE_DIR` is one.
fn words(text: &str) -> impl Iterator<Item = String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
}

/// `word` without an ending such as `ing` or `s`, so that `deploying` finds `deploy`
/// and `deployments`. It keeps three letters at least.
fn stem(word: &str) -> String {
    for ending in ["ing", "ed", "es", "s"] {
        if let Some(stem) = word.strip_suffix(ending)
            && stem.chars().count() >= 3
        {
            return stem.to_owned();
        }
    }
    word.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first(query: &str) -> (&'static str, Option<&'static str>) {
        let hit = search(query, 1, None).into_iter().next().expect("a passage matches");
        (hit.page.slug, hit.heading.map(|heading| heading.id.as_str()))
    }

    #[test]
    fn every_page_is_cut_at_its_headings() {
        let passages = passages();
        for page in pages() {
            let of_page: String = passages
                .iter()
                .filter(|passage| passage.page.slug == page.slug)
                .map(|passage| passage.markdown)
                .collect();
            assert_eq!(of_page.trim(), page.markdown.trim(), "{} lost some text", page.slug);
        }
        let headings: usize = pages().map(|page| page.rendered().headings.len()).sum();
        assert!(passages.len() >= headings);
    }

    #[test]
    fn finds_the_passage_about_what_is_searched() {
        assert_eq!(first("OXPLOY_HTTPS_PORT"), ("configuration", Some("oxploy")));
        assert_eq!(first("systemd service"), ("installation", Some("run-it-as-a-service")));
        assert_eq!(first("how do I back up my sqlite database"), ("storage", Some("backups")));
        assert_eq!(first("private repository"), ("github", Some("private-repositories")));
    }

    #[test]
    fn hits_link_to_the_site_absolutely() {
        let hits = search("environment variables storage", 10, None);
        assert!(!hits.is_empty());
        for hit in hits {
            assert!(!hit.markdown.contains("](/"), "{}", hit.markdown);
        }
    }

    #[test]
    fn finds_nothing_without_a_word() {
        assert!(search("", 5, None).is_empty());
        assert!(search("? ! a", 5, None).is_empty());
        assert!(search("zzqxv", 5, None).is_empty());
    }

    #[test]
    fn takes_each_word_once_without_its_ending() {
        assert_eq!(terms("Deploying deploys, the DOMAINS a"), ["deploy", "the", "domain"]);
        assert_eq!(stem("uses"), "use");
        assert_eq!(stem("bus"), "bus");
        assert_eq!(terms(&"word ".repeat(30)).len(), 1);
    }

    #[test]
    fn leaves_the_pictures_out() {
        assert_eq!(without_pictures("Before.\n\n![The Domains tab](domains)\n\nAfter."), "Before.\n\n\n\nAfter.");
        assert_eq!(without_pictures("No picture."), "No picture.");
    }

    #[test]
    fn gives_no_more_than_asked() {
        assert!(search("the", 3, None).len() <= 3);
    }
}
