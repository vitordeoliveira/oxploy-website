oxploy reads a few environment variables when it starts, such as the `Environment=`
lines of its [systemd service](/docs/installation#run-it-as-a-service).

## oxploy

| Variable | Default | What it sets |
| --- | --- | --- |
| `DATABASE_URL` | `sqlite:data.db` | oxploy's own database, a SQLite file. |
| `OXPLOY_DATA_DIR` | `data` | Where the apps' code, builds, logs and storage live. |
| `HOST` | `127.0.0.1` | The address the panel listens at. |
| `PORT` | `3000` | The port the panel listens at. Caddy reaches it there. |
| `OXPLOY_CADDY_ADMIN` | `127.0.0.1:2019` | Caddy's admin API. |
| `OXPLOY_HTTP_PORT` | `80` | The port Caddy serves HTTP at. |
| `OXPLOY_HTTPS_PORT` | `443` | The port Caddy serves HTTPS at. |
| `OXPLOY_PUBLIC_IP` | found on the machine | The server's public IPv4 addresses, separated by commas, for servers behind NAT. |

Relative paths start from oxploy's working directory.

## What apps get

oxploy starts each app with only these variables, and keeps its own, such as
`DATABASE_URL`, to itself:

| Variable | What it holds |
| --- | --- |
| `PORT` | The port the app must listen at, on `127.0.0.1`. |
| `OXPLOY_STORAGE_DIR` | The app's [storage folder](/docs/storage). |
| Database variables | Each [database](/docs/storage#sqlite-databases) with a variable. |
| Your variables | The service's [environment](/docs/deploying#environment-variables). |
| `PATH`, `HOME`, `USER`, `LOGNAME`, `LANG`, `LC_ALL`, `TZ`, `TMPDIR` | As oxploy has them. |

Builds get your variables and the databases', with the ones that find Rust's toolchains,
such as `CARGO_HOME` and `RUSTUP_HOME`, but not `PORT` or `OXPLOY_STORAGE_DIR`. A build
script also gets `OXPLOY_OUTPUT_DIR`, the folder to leave the binary in. The script after
the build gets what the app gets, with the new binary on the `PATH`.
