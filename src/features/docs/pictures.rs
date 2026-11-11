//! The screenshots of the panel the pages show, in `assets/docs/`. A page shows one as
//! `![What it shows](name)`, and gets the light one or the dark one as the reader's
//! theme is.
//!
//! Each was taken at twice the scale, in both themes, of a panel with only example data,
//! and saved as `<name>-light.webp` and `<name>-dark.webp`. A new one also goes in
//! [`all`], with its size on the page.

use topcoat::asset::{Asset, AssetConfig, asset};

use crate::features::seo;

/// A screenshot, in the light theme and in the dark one.
pub struct Picture {
    pub name: &'static str,
    pub light: Asset,
    pub dark: Asset,
    /// Its size on the page, in CSS pixels: half the screenshot's.
    pub width: u32,
    pub height: u32,
}

/// A screenshot in `assets/docs/`, by its name and its size on the page.
macro_rules! picture {
    ($name:literal, $width:literal x $height:literal) => {
        Picture {
            name: $name,
            light: asset!(concat!("assets/docs/", $name, "-light.webp")),
            dark: asset!(concat!("assets/docs/", $name, "-dark.webp")),
            width: $width,
            height: $height,
        }
    };
}

/// Every screenshot.
pub fn all() -> [Picture; 19] {
    [
        picture!("setup", 440 x 724),
        picture!("project", 1024 x 760),
        picture!("new-service", 512 x 486),
        picture!("build-cargo", 712 x 523),
        picture!("build-release", 712 x 581),
        picture!("environment", 712 x 1402),
        picture!("deploy-card", 712 x 217),
        picture!("domains", 712 x 368),
        picture!("storage", 714 x 959),
        picture!("monitoring-app", 760 x 1184),
        picture!("monitoring-server", 714 x 611),
        picture!("settings", 714 x 574),
        picture!("settings-version", 712 x 261),
        picture!("mcp-page", 760 x 967),
        picture!("mcp-server-token", 512 x 430),
        picture!("mcp-org-token", 512 x 776),
        picture!("app-mcp", 714 x 832),
        picture!("app-mcp-tools", 712 x 582),
        picture!("external-mcp", 512 x 782),
    ]
}

/// The screenshot named `name`.
pub fn find(name: &str) -> Option<Picture> {
    all().into_iter().find(|picture| picture.name == name)
}

/// Where the HTML of a page puts a screenshot's address, until it is served, since the
/// assets' addresses are only known then.
fn marker(name: &str, theme: &str) -> String {
    format!("oxploy-picture:{name}:{theme}")
}

/// The HTML that shows `picture`, described as `alt`, with markers for its addresses.
pub fn html(picture: &Picture, alt: &str) -> String {
    format!(
        r#"<picture><source media="(prefers-color-scheme: dark)" srcset="{dark}"><img src="{light}" alt="{alt}" width="{width}" height="{height}" loading="lazy" decoding="async"></picture>"#,
        dark = marker(picture.name, "dark"),
        light = marker(picture.name, "light"),
        alt = escape(alt),
        width = picture.width,
        height = picture.height,
    )
}

/// `html` with the addresses `assets` serves the screenshots named in `names` at.
pub fn resolve(html: &str, names: &[&'static str], assets: &AssetConfig) -> String {
    let mut html = html.to_owned();
    for picture in all().into_iter().filter(|picture| names.contains(&picture.name)) {
        html = html
            .replace(&marker(picture.name, "light"), &assets.resolve(picture.light))
            .replace(&marker(picture.name, "dark"), &assets.resolve(picture.dark));
    }
    html
}

/// `markdown` with each screenshot it shows at its absolute address on the site, in
/// the light theme, for those who read the Markdown elsewhere. Without `assets`, the
/// names stay.
pub fn absolute(markdown: &str, assets: Option<&AssetConfig>) -> String {
    let Some(assets) = assets else {
        return markdown.to_owned();
    };
    let mut markdown = markdown.to_owned();
    for picture in all() {
        let named = format!("]({})", picture.name);
        if markdown.contains(&named) {
            let address = seo::url(&assets.resolve(picture.light));
            markdown = markdown.replace(&named, &format!("]({address})"));
        }
    }
    markdown
}

/// `text` with the characters HTML gives a meaning escaped.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_files_exist() {
        let pictures = all();
        for picture in &pictures {
            assert_eq!(pictures.iter().filter(|other| other.name == picture.name).count(), 1, "{}", picture.name);
            for theme in ["light", "dark"] {
                let path = format!("{}/assets/docs/{}-{theme}.webp", env!("CARGO_MANIFEST_DIR"), picture.name);
                assert!(std::path::Path::new(&path).exists(), "{path} is missing");
            }
        }
    }

    #[test]
    fn shows_the_theme_of_the_reader() {
        let html = html(&find("setup").unwrap(), r#"The "first run" page"#);
        assert!(html.contains(r#"media="(prefers-color-scheme: dark)" srcset="oxploy-picture:setup:dark""#));
        assert!(html.contains(r#"src="oxploy-picture:setup:light""#));
        assert!(html.contains(r#"alt="The &quot;first run&quot; page""#));
        assert!(html.contains(r#"width="440" height="724""#));
    }

    #[test]
    fn markdown_keeps_names_without_assets() {
        assert_eq!(absolute("![The setup](setup)", None), "![The setup](setup)");
    }
}
