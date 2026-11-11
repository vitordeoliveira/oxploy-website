# oxploy website

The site of [oxploy](https://github.com/vitordeoliveira/oxploy): its home page and its
documentation. It is a [Topcoat](https://github.com/tokio-rs/topcoat) app, with the same
theme as the panel, and a SQLite database for its visits.

## Development

Install the Topcoat CLI, then start the development server, which rebuilds the site when
you edit it:

```sh
cargo install topcoat-cli
topcoat dev
```

Then open <http://127.0.0.1:3000>. Set `PORT` to use another port, such as when the panel
runs on 3000.

## Writing the documentation

Each page is a Markdown file in `docs/`, without a `#` title: the title and the sentence
under it come from the list in `src/features/docs.rs`, which also orders the sidebar. To
add a page, write `docs/<slug>.md` and add it to that list; it opens at `/docs/<slug>`.
The list also gives each page what search results show: a `search_title`, which says what
someone would search for and fits in 60 characters with ` · oxploy docs` after it, and a
`description` of 70 to 160 characters. The tests check both.

`##` and `###` headings get anchors, such as `/docs/installation#run-it-as-a-service`, and
make the page's table of contents. Link to other pages with their address, such as
`[Domains](/docs/domains)`. The documentation describes what oxploy does: check what a page
says against oxploy's code.

A page shows a screenshot of the panel as `![What it shows](name)`, from
`assets/docs/<name>-light.webp` and `assets/docs/<name>-dark.webp`, and readers get the one
of their theme. Each is taken at twice the scale, in a window 1024 pixels wide so its text
stays legible in the page, and listed with its size on the page, half the screenshot's, in
`src/features/docs/pictures.rs`. Take them on a panel with only example data, never a real
server's: the panel shows addresses, paths and variables. The tests check that each image a
page shows is there in both themes.

## Search engines

Every page gives search engines and link previews the address `https://oxploy.com`,
`ORIGIN` in `src/features/seo.rs`, whichever address it was opened at.

Each page has a canonical address without its query, so a link's `utm_source` does not make
it a second page, and the pages that are not the site's, such as `/admin`, are `noindex`.
`/sitemap.xml` lists the pages in `listed_pages` in `src/app.rs`, which a new page joins,
and `/robots.txt` points to it. The home page describes oxploy and its FAQ to search engines
in JSON-LD, and each documentation page itself as an article with its breadcrumbs.

Link previews show `assets/preview.jpg`, a 1200 × 630 picture in the dark theme's colors.

## Visits and the admin

The site counts the visits to its pages without keeping anyone's address: a visitor is a
hash of their address and browser with a secret key and the day, so the same person counts
once a day and cannot be followed from one day to the next. Each visit also keeps the kind
of device, the browser's family and the link's `utm_source`. Programs, such as search
engines, are left out, and so are the admin and the privacy page. Only the site's pages
count, those in `listed_pages` in `src/app.rs`.

A banner asks each visitor whether the site may keep a cookie. Those who accept get a
random ID in it for 13 months, which tells when they come back on another day; those who
decline are still counted, with nothing kept in their browser. The choice itself is kept
in a cookie too, and `/privacy` explains all of it and lets visitors change it.

`/admin` shows the visits, for the last 7, 30 or 90 days: visitors, page views, each
day's, the pages most opened, where visitors came from, their devices, browsers and
campaigns, and, among those who accepted the cookie, how many came back. Its name and
password are `ADMIN_USERNAME` and `ADMIN_PASSWORD`; without them, nobody can sign in.
Days are in the server's time zone, which `TZ` changes, such as `TZ=America/Sao_Paulo`.

The visits are in a SQLite database: `DATABASE_URL`, or else `website.db` in the storage
folder oxploy gives the site in `OXPLOY_STORAGE_DIR`, which deploys keep, or else
`data.db` where the site runs. It applies its migrations, in `toasty/`, when it starts.
After changing a model, generate one with
`cargo run -- toasty migration generate --name <what_changed>`. Visits older than 400
days are forgotten.

## MCP

AI agents read the documentation through the [Model Context Protocol](https://modelcontextprotocol.io),
at two addresses. `/mcp` is public, so anyone's agent can search and read the pages:

| Tool | What it does |
| --- | --- |
| `search_docs` | The passages that best match a query, each a page's part under one heading, in Markdown, with its address. |
| `read_doc` | A whole page, in Markdown, with its links made absolute on `https://oxploy.com`. |
| `list_docs` | Every page, as the sidebar orders them, with their headings. |

```sh
claude mcp add --transport http oxploy-docs https://oxploy.com/mcp
```

`/admin/mcp` is the owner's, and also has `site_visits`, the visits of the last 1 to 90
days as `/admin` shows them. It takes the token in `OXPLOY_MCP_TOKEN`, sent as
`Authorization: Bearer <token>`, and a missing or wrong one gets a 401 after a second;
without the variable, nobody gets in. oxploy reaches it: turn MCP on in the site's
**MCP** tab at the path `/admin/mcp`, not the default `/mcp`, since Caddy then hides the
path from the site's domains, and oxploy gives the site the token and passes its tools
on to the organization's tokens.

Both answer each JSON-RPC message, sent by POST, with plain JSON, with no session and
no event stream, in `src/app/mcp.rs`. Each tool is a file of its own in `src/app/mcp/`,
with a line in `definitions` and in `call_tool`. The search, in
`src/features/docs/search.rs`, cuts the pages at their `##` and `###` headings, and
weighs the words of a query by how few passages have them.

## Deploying

The site deploys with oxploy itself, as a service built with cargo: Topcoat apps get their
assets bundled at each deploy. It listens on `127.0.0.1` at `PORT`, as oxploy expects.

## Project structure

| Path | Contents |
| --- | --- |
| `src/app.rs` | The router, the layout every page shares, the home page, and the sitemap. |
| `src/app/docs.rs`, `src/app/docs/` | The documentation's layout, with its sidebar, and its pages. |
| `src/app/admin.rs` | The admin's page: signing in, and the visits. |
| `src/app/mcp.rs`, `src/app/mcp/`, `src/app/admin/mcp.rs` | The MCP servers, at `/mcp` and `/admin/mcp`, and their tools. |
| `src/app/privacy.rs` | The privacy page, and the cookie banner. |
| `src/features/docs.rs`, `src/features/docs/` | The documentation's pages, their order, their rendering from Markdown, their screenshots, and their search. |
| `src/features/seo.rs` | The site's address, its `robots.txt`, and the structured data in the pages. |
| `src/features/analytics.rs`, `src/features/admin.rs`, `src/features/mcp.rs` | The visits, the admin's sign-in, and the owner's MCP token. |
| `src/db.rs`, `toasty/` | The database, and its migrations. |
| `src/components/` | Shared UI components. |
| `docs/` | The documentation's pages, in Markdown. |
| `assets/docs/` | The screenshots the pages show, in the light and the dark theme. |
| `styles.css` | The theme, and the styles of the documentation's pages. |
