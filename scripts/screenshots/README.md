# Screenshots

The documentation's screenshots of the panel come from a demo panel with only example
data: the Acme organization, its `shop` project, and a `docs` service that runs this site
and serves its MCP. `run.sh` does all of it:

```sh
scripts/screenshots/run.sh
```

It builds oxploy, from `../oxploy` or `OXPLOY_DIR`, and this site, starts a panel of its
own in a temporary folder, fills it through its pages, waits for the charts to fill,
takes each screenshot in the light and the dark theme, and writes them to `assets/docs`
as WebP, with their sizes in `src/features/docs/pictures.rs`. It stops what it started
when it ends.

It needs `cargo`, the Topcoat CLI, Node, `sqlite3`, `cwebp` and a Chromium, such as
Chrome or Brave, or the one in `BROWSER`. `PANEL_PORT`, `FRESH_PORT` and `APP_PORT`
change its ports, 3950, 3951 and 10050, and `MONITOR_WAIT` how long the charts fill, 180
seconds.

## Nothing of this machine

The panel shows the machine it runs on: its addresses, its folders, its environment and
its system. `capture.mjs` puts a server installed as the docs say in their place, such
as `203.0.113.7` and `/var/lib/oxploy`, and leaves out the machine's name and processor,
and the demo's panel has its own database, so no real service, token or variable shows.
Still, look at each screenshot before you commit it.

Never take them from a real panel.

## A new screenshot

Add it to `shots` in `capture.mjs`, and to the list in `src/features/docs/pictures.rs`
with any size, which `run.sh` corrects. A page shows it as `![What it shows](name)`.
`node capture.mjs <name>` takes only the ones named, of a demo panel already seeded.
