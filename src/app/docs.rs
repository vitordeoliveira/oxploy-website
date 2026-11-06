//! The documentation: a sidebar of its pages beside the one being read.

pub mod slug;

use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::{Slot, error::redirect_permanent, layout, page, request::uri},
    view::{View, attributes, component, view},
};

use crate::features::docs::{self, SECTIONS};

/// `/docs` opens the first page, for good, so search engines list that page instead.
#[page]
pub async fn page() -> Result<()> {
    Err(redirect_permanent(format!("/docs/{}", docs::first().slug)).into())
}

#[layout]
async fn layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let current = uri(cx).path().strip_prefix("/docs/").unwrap_or("").to_owned();

    Ok(view! {
        <div class="mx-auto flex w-full max-w-6xl flex-1 flex-col gap-8 px-4 py-8 sm:px-6 lg:flex-row lg:gap-12 lg:py-12">
            // Phones and tablets get the pages in a list that opens.
            <details class="group rounded-lg border border-border lg:hidden">
                <summary class="flex cursor-pointer list-none items-center justify-between px-4 py-3 text-sm font-medium [&::-webkit-details-marker]:hidden">
                    "Documentation"
                    icon(data: iconify_icon!("lucide:chevron-down"), size: 16, attrs: attributes! { class="text-muted-foreground transition-transform group-open:rotate-180" })
                </summary>
                <div class="border-t border-border px-2 py-3">
                    navigation(current: &current)
                </div>
            </details>
            <aside class="hidden w-56 shrink-0 lg:block">
                <div class="sticky top-24">
                    navigation(current: &current)
                </div>
            </aside>
            (slot)
        </div>
    })
}

/// The documentation's pages, by section, with the one at `current` marked.
#[component]
async fn navigation(current: &str) -> Result<impl View> {
    Ok(view! {
        <nav class="flex flex-col gap-6" aria-label="Documentation">
            #[key(section.title)]
            for section in SECTIONS {
                <div class="flex flex-col gap-1">
                    <p class="px-2 pb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">(section.title)</p>
                    #[key(entry.slug)]
                    for entry in section.pages {
                        <a
                            href=(format!("/docs/{}", entry.slug))
                            class=(if entry.slug == current {
                                "rounded-md bg-primary/10 px-2 py-1.5 text-sm font-medium text-primary"
                            } else {
                                "rounded-md px-2 py-1.5 text-sm text-muted-foreground hover:bg-foreground/5 hover:text-foreground"
                            })
                            aria-current=((entry.slug == current).then_some("page"))
                        >
                            (entry.title)
                        </a>
                    }
                </div>
            }
        </nav>
    })
}
