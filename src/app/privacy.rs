//! What the site keeps about its visitors, at `/privacy`, and the banner that asks
//! each visitor whether the site may keep its cookie.

use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        page,
        request::uri,
        route,
    },
    runtime::{Event, procedure, signal},
    view::{View, component, view},
};

use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::features::analytics;

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let choice = analytics::choice(cx);
    let status = match choice {
        Some(true) => "You accepted the cookie.",
        Some(false) => "You declined the cookie.",
        None => "You have not chosen yet.",
    };

    Ok(view! {
        <main class="mx-auto flex w-full max-w-3xl flex-col gap-10 px-4 py-14 sm:px-6">
            <div class="flex flex-col gap-3">
                <h1 class="text-3xl font-semibold tracking-tight">"Privacy"</h1>
                <p class="text-pretty text-muted-foreground">
                    "This site counts its visits to learn which pages people read and where they come from. It counts them on its own server, with no other company involved, and shares them with no one."
                </p>
            </div>

            <section class="flex flex-col gap-3">
                <h2 class="text-xl font-semibold tracking-tight">"What every visit keeps"</h2>
                <p class="text-muted-foreground">"For each page you open, with or without the cookie:"</p>
                <ul class="flex list-disc flex-col gap-1.5 pl-5 text-muted-foreground">
                    <li>"The page, and when you opened it."</li>
                    <li>"The site you came from, by its name only, such as github.com."</li>
                    <li>"Whether you are on a phone, a tablet or a computer, and your browser's name, such as Firefox."</li>
                    <li>"The " <code class="font-mono text-sm text-foreground">"utm_source"</code> " of the link you followed, if it has one, such as newsletter."</li>
                </ul>
                <p class="text-muted-foreground">
                    "You are counted as a code made from your address and your browser, mixed with a secret and the day. The code changes every day, so your visits on different days cannot be tied together, and your address is not kept. Programs, such as search engines, are not counted. Visits are deleted after 400 days."
                </p>
            </section>

            <section class="flex flex-col gap-3">
                <h2 class="text-xl font-semibold tracking-tight">"The install script"</h2>
                <p class="text-muted-foreground">
                    "When a server fetches " <code class="font-mono text-sm text-foreground">"install.sh"</code>
                    " with curl or wget, to install oxploy, the site counts it, with when it was fetched and a code made from the server's address and its program, mixed with a secret and the day, as for a visit. Nothing else is kept, and the counts are deleted after 400 days too."
                </p>
            </section>

            <section class="flex flex-col gap-3">
                <h2 class="text-xl font-semibold tracking-tight">"What the cookie adds"</h2>
                <p class="text-muted-foreground">
                    "If you accept, the site keeps a cookie named " <code class="font-mono text-sm text-foreground">"visitor"</code>
                    " in your browser for 13 months, with a random number in it. It tells the site that you came back on another day, so it can count the people who return. The site only reads it on its own pages, and stores a code made from it, never the number itself."
                </p>
                <p class="text-muted-foreground">
                    "A cookie named " <code class="font-mono text-sm text-foreground">"cookies"</code>
                    " remembers your choice, so the site does not ask again. If you decline, it is the only one the site keeps."
                </p>
            </section>

            <section class="flex flex-col gap-4 rounded-xl border border-border bg-card p-5">
                <div class="flex flex-col gap-1">
                    <h2 class="font-semibold">"Your choice"</h2>
                    <p class="text-sm text-muted-foreground">(status) " You can change it here at any time."</p>
                </div>
                <form method="post" action="/privacy/choose" class="flex flex-wrap gap-2">
                    <input type="hidden" name="back" value="/privacy">
                    <button type="submit" name="choice" value="no" class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))>
                        if choice == Some(false) { "Declined" } else { "Decline" }
                    </button>
                    <button type="submit" name="choice" value="yes" class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))>
                        if choice == Some(true) { "Accepted" } else { "Accept" }
                    </button>
                </form>
            </section>
        </main>
    })
}

/// The banner at the foot of the page that asks whether the site may keep its cookie,
/// until the visitor answers. With the browser runtime, the answer goes without
/// opening the page again, which would count it twice; without it, the form sends it.
#[component]
pub async fn cookie_banner(cx: &Cx) -> Result<impl View> {
    let back = uri(cx)
        .path_and_query()
        .map_or("/".to_owned(), |path| path.as_str().to_owned());
    let answered = signal(cx, || false);

    Ok(view! {
        <div :hidden=$(answered.get()) class="fixed inset-x-0 bottom-0 z-50 p-3 sm:p-5">
            <form
                method="post"
                action="/privacy/choose"
                aria-label="Cookie"
                class="mx-auto flex max-w-3xl flex-col gap-4 rounded-xl border border-border bg-card p-4 shadow-lg sm:flex-row sm:items-center sm:p-5"
            >
                <input type="hidden" name="back" value=(back)>
                <p class="text-sm text-pretty text-muted-foreground">
                    "May this site keep a cookie to know when you come back? It is only read here and never shared. If you decline, your visit is still counted, without anything kept in your browser. "
                    <a href="/privacy" class="font-medium text-foreground underline underline-offset-4">"What is kept"</a>
                </p>
                <div class="flex shrink-0 gap-2">
                    <button
                        type="submit"
                        name="choice"
                        value="no"
                        class=(button_variants(ButtonVariant::Outline, ButtonSize::Sm))
                        @click=$(async |e: Event| {
                            e.prevent_default();
                            choose_cookie(false).await;
                            answered.set(true);
                        })
                    >
                        "Decline"
                    </button>
                    <button
                        type="submit"
                        name="choice"
                        value="yes"
                        class=(button_variants(ButtonVariant::Primary, ButtonSize::Sm))
                        @click=$(async |e: Event| {
                            e.prevent_default();
                            choose_cookie(true).await;
                            answered.set(true);
                        })
                    >
                        "Accept"
                    </button>
                </div>
            </form>
        </div>
    })
}

/// Records the visitor's answer to the banner, from the browser runtime.
#[procedure]
async fn choose_cookie(cx: &Cx, accept: bool) -> Result<()> {
    analytics::choose(cx, accept);
    Ok(())
}

/// The visitor's answer, as the form sends it, and the page it was on.
#[derive(Deserialize)]
struct ChoiceForm {
    choice: String,
    back: Option<String>,
}

/// Records the visitor's answer, from a form, and goes back to the page it was on.
#[route(POST "./choose")]
async fn choose(cx: &Cx, Form(form): Form<ChoiceForm>) -> Result<SeeOther> {
    analytics::choose(cx, form.choice == "yes");
    Ok(see_other(own_path(form.back.as_deref())))
}

/// `back` if it is a path on this site, so the form cannot send anyone to another one,
/// or else the home page.
fn own_path(back: Option<&str>) -> String {
    match back {
        Some(path) if path.starts_with('/') && !path.starts_with("//") && !path.starts_with("/\\") => path.to_owned(),
        _ => "/".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_go_back_only_to_this_site() {
        assert_eq!(own_path(Some("/docs/domains?utm_source=x")), "/docs/domains?utm_source=x");
        assert_eq!(own_path(Some("//evil.example")), "/");
        assert_eq!(own_path(Some("/\\evil.example")), "/");
        assert_eq!(own_path(Some("https://evil.example")), "/");
        assert_eq!(own_path(None), "/");
    }
}
