A service is one app: a repository and branch that oxploy builds into a binary and runs.
Create a project under **Projects**, then a service in it.

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

By default oxploy runs `cargo build --release` in the build path, with these options:

- **Rust version**: a channel such as `stable`, or a release such as `1.85`. Left empty,
  the repository's toolchain file decides, or else `stable`.
- **Binary**: which binary to build, for packages with more than one.
- **Features**: separated by commas, such as `postgres, tls`.

The panel shows the cargo command these options make. [Topcoat](https://github.com/tokio-rs/topcoat)
apps also get their assets bundled, as `topcoat asset bundle` does.

To build another way, choose a **build script** instead. It runs in the build path, and
must leave the binary in the folder in `OXPLOY_OUTPUT_DIR`. Give the binary's file name
beside the script, and oxploy starts that file.

## Environment variables

The **Environment** tab holds the service's variables, one `NAME=value` per line. cargo
gets them while it builds, and the binary when it runs.

```sh
RUST_LOG=info
DATABASE_URL=sqlite:${OXPLOY_STORAGE_DIR}/data.db
PUBLIC_URL=https://shop.example.com
```

A value can use what oxploy sets, such as `${PORT}` and `${OXPLOY_STORAGE_DIR}`, and the
lines above it. A value in single quotes stays as it is. Names that start with `OXPLOY_`
are oxploy's, and so is `PORT`.

## Run a script after the build

In the **Advanced** tab, a script can run after each build, before the new release
starts, such as one that migrates the database:

```sh
shop-api migrate
```

It runs with `sh -e` where the app runs, with the app's variables, its databases linked,
and the new binary on the `PATH`. If it fails, the deploy stops and the running release
stays.

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
The **Logs** tab shows what the app prints, and notes each start and exit. **Stop** keeps
the app stopped until you start it or deploy again, and **Restart** starts the live
release again.
