# oxploy website

The site of [oxploy](https://github.com/vitordeoliveira/oxploy): its home page and its
documentation. It is a [Topcoat](https://github.com/tokio-rs/topcoat) app, with the same
theme as the panel, and needs no database.

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

`##` and `###` headings get anchors, such as `/docs/installation#run-it-as-a-service`, and
make the page's table of contents. Link to other pages with their address, such as
`[Domains](/docs/domains)`. The documentation describes what oxploy does: check what a page
says against oxploy's code.

## Deploying

The site deploys with oxploy itself, as a service built with cargo: Topcoat apps get their
assets bundled at each deploy. It listens on `127.0.0.1` at `PORT`, as oxploy expects.

## Project structure

| Path | Contents |
| --- | --- |
| `src/app.rs` | The router, the layout every page shares, and the home page. |
| `src/app/docs.rs`, `src/app/docs/` | The documentation's layout, with its sidebar, and its pages. |
| `src/features/docs.rs` | The documentation's pages, their order, and their rendering from Markdown. |
| `src/components/` | Shared UI components. |
| `docs/` | The documentation's pages, in Markdown. |
| `styles.css` | The theme, and the styles of the documentation's pages. |
