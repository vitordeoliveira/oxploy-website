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
| `OXPLOY_CADDY_LOGS` | `127.0.0.1:2020` | Where Caddy sends, over UDP, the requests apps answered with a server error, which oxploy writes in their [logs](/docs/deploying#keep-it-running). |
| `OXPLOY_HTTP_PORT` | `80` | The port Caddy serves HTTP at. |
| `OXPLOY_HTTPS_PORT` | `443` | The port Caddy serves HTTPS at. |
| `OXPLOY_PUBLIC_IP` | found on the machine | The server's public IPv4 addresses, separated by commas, for servers behind NAT. |
| `OXPLOY_TELEMETRY` | on | `off` stops the daily [usage report](/docs/usage-reporting), whatever Settings says. `DO_NOT_TRACK=1` does the same. |

Relative paths start from oxploy's working directory. The time zone the panel shows times
in is not a variable: [Settings picks it](/docs/installation#pick-the-time-zone).

## What apps get

oxploy starts each app with only these variables, and keeps its own, such as
`DATABASE_URL`, to itself:

| Variable | What it holds |
| --- | --- |
| `PORT` | The port the app must listen at, on `127.0.0.1`, or in a [Docker app's](/docs/deploying#docker-apps) container, its container port. |
| `HOST` | For a Docker app only: `0.0.0.0`, every address of its container. |
| `OXPLOY_STORAGE_DIR` | The app's [storage folder](/docs/storage). |
| `OXPLOY_MCP_URL` | Where this server's [MCP](/docs/mcp) answers on `127.0.0.1`, for an app that is a client of its organization's MCP. |
| `OXPLOY_MCP_TOKEN` | The token oxploy calls the [app's MCP server](/docs/app-mcp) with, while its MCP tab has it on. |
| MCP tokens | An organization's [token made available to apps](/docs/mcp#tokens-for-apps), which a line such as `MCP_TOKEN=${OXPLOY_MCP_TELEGRAM_BOT}` gives the app. |
| Other apps of the project | Where [another app of the project](/docs/deploying#reach-another-app-of-the-project) answers, which a line such as `API_URL=${OXPLOY_SHOP_API_URL}` gives the app. |
| Database variables | Each [database](/docs/storage#sqlite-databases) with a variable. |
| Your variables | The service's [environment](/docs/deploying#environment-variables). |
| `PATH`, `HOME`, `USER`, `LOGNAME`, `LANG`, `LC_ALL`, `TZ`, `TMPDIR` | As oxploy has them, except for a Docker app, whose image sets its own. |

The service's **Environment** tab lists all of them, as [the app gets
them](/docs/deploying#environment-variables).

Builds get your variables and the databases', with the ones that find Rust's toolchains,
such as `CARGO_HOME` and `RUSTUP_HOME`, but not `PORT` or `OXPLOY_STORAGE_DIR`. A build
script also gets `OXPLOY_OUTPUT_DIR`, the folder to leave the binary in. The script after
the build gets what the app gets, with the new binary on the `PATH`.
