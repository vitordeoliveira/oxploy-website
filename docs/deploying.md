A service is one app: a repository and branch that oxploy builds into a binary and runs.
Create a project under **Projects**, then a service in it with **New service**. A
service is a **Rust app**, which this page is about, or an [external MCP
server](/docs/app-mcp#external-mcp-servers), which runs elsewhere.

![The New service dialog, with a Rust app named shop-web](new-service)

## What the app must do

oxploy starts the binary and passes it a port in `PORT`. A web app must listen on
`127.0.0.1` at that port, where Caddy reaches it:

```rust
let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_owned());
let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
```

The service keeps its port, from 10000 to 19999, from its first deploy on. An app that
never listens, such as a worker, still runs; its deploy notes that nothing answers.

## Choose the source

In the service's **General** tab, choose the GitHub account, the repository and the branch,
and the **build path**: the folder the build runs in, such as the one with the
`Cargo.toml`, for a workspace or a repository with more than one app.

## Choose how it builds

The **Build** card has three ways to get the binary, one tab each. Saving a tab switches
the service to it.

![The Build card's Cargo tab, with its Rust version, binary and features, and the command they make](build-cargo)

**Cargo**, the default, runs `cargo build --release` in the build path, with these
options:

- **Rust version**: a channel such as `stable`, or a release such as `1.85`. Left empty,
  the repository's toolchain file decides, or else `stable`.
- **Binary**: which binary to build, for packages with more than one.
- **Features**: separated by commas, such as `postgres, tls`.

The card shows the cargo command these options make. [Topcoat](https://github.com/tokio-rs/topcoat)
apps also get their assets bundled, as `topcoat asset bundle` does.

**Script** builds another way. It runs in the build path, and must leave the binary in the
folder in `OXPLOY_OUTPUT_DIR`. Give the binary's file name beside the script, and oxploy
starts that file.

**GitHub release** builds nothing on the server: see the next section.

## Deploy GitHub releases

A small server builds slowly. A service can instead deploy a file of its repository's
latest GitHub release, built elsewhere, such as in GitHub Actions. The branch and the
build path do not apply then.

![The Build card's GitHub release tab, with a file name, its binary and Deploy new releases on](build-release)

- **File**: the release's file, chosen from the latest release's, or typed. A `*` stands
  for any text, such as the version in `shop-api-*-x86_64-linux.tar.gz`.
- **Binary**: the file is the binary itself, or a `.tar.gz` with it inside; then choose
  the binary among the files in the archive that can run, such as `shop-api` or
  `bin/shop-api`. It runs with the files beside it, such as a Topcoat app's assets.
- **Deploy new releases**: oxploy looks at the repository every minute, and deploys a new
  release once its file is there. A release that fails to deploy is not tried again on
  its own.

oxploy downloads the file through the GitHub App, and checks it against the release's
`.sha256` of the file when there is one. A binary built for another machine, such as a
macOS one on a Linux server, fails its deployment before the running release stops. The
tab says which machine the server is, such as `x86_64 Linux`.

## Environment variables

The **Environment** tab holds the service's variables, one `NAME=value` per line, as in a
`.env` file, where a line that starts with `#` is a comment. cargo gets them while it
builds, and the binary when it runs.

```sh
RUST_LOG=info
DATABASE_URL=sqlite:${OXPLOY_STORAGE_DIR}/data.db
PUBLIC_URL=https://shop.example.com
```

A value can use what oxploy sets, such as `${PORT}` and `${OXPLOY_STORAGE_DIR}`, and the
lines above it. A value in single quotes stays as it is. Names that start with `OXPLOY_`
are oxploy's, and so is `PORT`.

Below the editor, **What the app gets** lists every variable the app gets when it runs,
with `${NAME}` filled in, grouped by where each comes from: yours, the ones oxploy sets,
the [databases](/docs/storage#sqlite-databases) of the Storage tab, the project's other
apps, the organization's [MCP tokens](/docs/mcp#tokens-for-apps) it can use, and the few
oxploy passes on from the server, such as `PATH`. The values that may be secret stay
blurred until you show them.

![What the app gets, with the variables grouped by where they come from](environment)

Builds get the same, except `PORT` and `OXPLOY_STORAGE_DIR`, which reach only the app and
the script after the build. See [Configuration](/docs/configuration#what-apps-get).

### Reach another app of the project

A value can also use where another app of the same project answers. Each app has three
variables, named after its app name, which comes from the project's and the service's
names when the service is made and stays the same if you rename them:

```sh
# The project Shop has the apps API and Web. In Web's Environment tab:
API_URL=${OXPLOY_SHOP_API_URL}
API_HOST=${OXPLOY_SHOP_API_HOST}
API_PORT=${OXPLOY_SHOP_API_PORT}
```

Web then gets `API_URL=http://127.0.0.1:10002`, `API_HOST=127.0.0.1` and
`API_PORT=10002`. An app gets them only from a line that uses them, so it reads them
under the names its code expects. An app has its port from the moment it is made, so
another app can use it before its first deploy. An app of another project is not offered,
and saving a line that uses one says so. If the app is deleted later, the variables that
used it are empty.

## Run a script after the build

In the **Advanced** tab, a script can run after each build, before the new release
starts, such as one that migrates the database:

```sh
shop-api migrate
```

It runs with `sh -e` where the app runs, with the app's variables, its databases linked,
and the new binary on the `PATH`, so it calls the binary by its name, without `./`. If it
fails, the deploy stops and the running release stays.

## Deploy

Press **Deploy**. The **Deployments** tab lists each deployment with its log, which grows
as it runs:

```text
==> Fetching acme/shop-api at main
==> Building with cargo build --release --bin shop-api
==> Running the script after the build
Stopped release #11
==> Starting on 127.0.0.1:10000
Listening on 127.0.0.1:10000
==> Deployed
```

One build runs at a time on the server; a deployment that has to wait says so. The first
time a service goes live, oxploy gives it [a domain at the server's
address](/docs/domains#an-address-for-every-new-app).

## Keep it running

If the app exits, oxploy starts it again, waiting longer after each crash, up to a minute.
**Stop** keeps the app stopped until you start it or deploy again, and **Restart** starts
the live release again. They run without reloading the page: the card says what each is
doing, then what became of it, such as *Restarted at 14:05:09*, or why it failed. The
status beside the service's name says since when the app runs.

![The Deploy card after a restart, with the live release](deploy-card)

The **Logs** tab shows what the app prints. The lines oxploy adds are in bold: each start
and exit, and the requests the app answered with a server error, which Caddy reports, as
an app may answer 500 without printing why, and Caddy answers 502 when the app does not
answer. Their queries are left out, as they may hold secrets.
