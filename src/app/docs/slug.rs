//! A page of the documentation, at `/docs/<slug>`.

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{error::RouterErrorExt, module_param, page, path_param},
    view::{Unescaped, View, component, view},
};

use crate::features::docs::{self, Page};

module_param!(pub slug: String, error = not_found);

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let slug = path_param::<Slug>(cx)?;
    let entry = docs::find(slug).ok_or_not_found()?;
    let rendered = entry.rendered();
    let (previous, next) = entry.neighbors();

    Ok(view! {
        <article class="min-w-0 flex-1">
            <p class="text-sm font-medium text-primary">(entry.section().title)</p>
            <h1 class="mt-2 text-3xl font-semibold tracking-tight sm:text-4xl">(entry.title)</h1>
            <p class="mt-3 text-lg text-pretty text-muted-foreground">(entry.summary)</p>
            // The Markdown in `docs/` is the site's own, so its HTML goes in as it is.
            <div class="prose mt-10">(Unescaped::new_unchecked(rendered.html.as_str()))</div>
            <nav class="mt-16 grid gap-4 border-t border-border pt-8 sm:grid-cols-2" aria-label="Read on">
                if let Some(previous) = previous {
                    neighbor(target: previous, label: "Previous")
                } else {
                    <span></span>
                }
                if let Some(next) = next {
                    neighbor(target: next, label: "Next")
                }
            </nav>
        </article>
        if !rendered.headings.is_empty() {
            <aside class="hidden w-52 shrink-0 xl:block">
                <nav class="sticky top-24 flex flex-col gap-2 text-sm" aria-label="On this page">
                    <p class="font-medium">"On this page"</p>
                    #[key(heading.id.clone())]
                    for heading in &rendered.headings {
                        <a
                            href=(format!("#{}", heading.id))
                            class=(if heading.nested { "pl-3 text-muted-foreground hover:text-foreground" } else { "text-muted-foreground hover:text-foreground" })
                        >
                            (&heading.text)
                        </a>
                    }
                </nav>
            </aside>
        }
    })
}

/// A link to the page before or after this one.
#[component]
async fn neighbor(target: &Page, label: &str) -> Result<impl View> {
    let next = label == "Next";

    Ok(view! {
        <a
            href=(format!("/docs/{}", target.slug))
            class=(if next {
                "group flex flex-col items-end gap-1 rounded-lg border border-border p-4 text-right hover:border-primary/50 sm:col-start-2"
            } else {
                "group flex flex-col gap-1 rounded-lg border border-border p-4 hover:border-primary/50"
            })
        >
            <span class="flex items-center gap-1 text-xs text-muted-foreground">
                if !next {
                    icon(data: iconify_icon!("lucide:arrow-left"), size: 12)
                }
                (label)
                if next {
                    icon(data: iconify_icon!("lucide:arrow-right"), size: 12)
                }
            </span>
            <span class="font-medium group-hover:text-primary">(target.title)</span>
        </a>
    })
}
