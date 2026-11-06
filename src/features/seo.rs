//! What search engines and link previews read about the site: the address its pages
//! are at, its `robots.txt`, and the structured data the pages carry.

use serde_json::Value;

/// The site's name, as search results and link previews show it.
pub const SITE_NAME: &str = "oxploy";

/// The address the site is at, without a slash at the end. Every page names this one,
/// whichever address it was opened at.
pub const ORIGIN: &str = "https://oxploy.com";

/// The absolute address of `path` on the site, such as `https://oxploy.com/docs/domains`.
pub fn url(path: &str) -> String {
    format!("{ORIGIN}{path}")
}

/// The site's `robots.txt`: crawlers may read every page but the admin's, and find them
/// all in the sitemap.
pub fn robots() -> String {
    format!("User-agent: *\nDisallow: /admin\n\nSitemap: {}\n", url("/sitemap.xml"))
}

/// `data` as the content of a `<script type="application/ld+json">`. Its `<`, `>` and
/// `&` are escaped as JSON allows, so no text in it can close the script.
pub fn json_ld(data: &Value) -> String {
    data.to_string()
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn robots_points_to_the_sitemap() {
        assert_eq!(robots(), "User-agent: *\nDisallow: /admin\n\nSitemap: https://oxploy.com/sitemap.xml\n");
    }

    #[test]
    fn structured_data_cannot_close_its_script() {
        let data = serde_json::json!({ "text": "</script><script>alert(1)</script> & more" });
        let script = json_ld(&data);
        assert!(!script.contains('<') && !script.contains('>') && !script.contains('&'));
        assert_eq!(serde_json::from_str::<Value>(&script).unwrap(), data);
    }
}
