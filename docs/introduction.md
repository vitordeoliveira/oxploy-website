oxploy is a deploy panel for Rust apps that runs on a server of your own. It builds each
service from its GitHub repository with cargo, runs the binary as a plain process, and
gives it a domain with HTTPS through [Caddy](https://caddyserver.com). There is no Docker
and no configuration file to write: you do everything from the panel.

## How it fits together

Everything runs on one machine:

- **oxploy** is a single binary with a folder of assets beside it. It serves the panel,
  keeps its own data in a SQLite file, builds the apps and runs them.
- **Caddy** runs beside it, as a service of its own. oxploy gives it every route through
  its admin API, and Caddy gets each domain's certificate from Let's Encrypt.
- **Your apps** run as processes listening on `127.0.0.1`, each at the port oxploy gives
  it in `PORT`. Caddy sends each domain to its app.

```text
browser ──HTTPS──▶ Caddy :443 ──▶ panel     127.0.0.1:3000
                              ├─▶ shop-api  127.0.0.1:10000
                              └─▶ blog      127.0.0.1:10001
```

Each app runs in a process group of its own, so it keeps running while oxploy stops,
restarts or updates itself. oxploy writes down each app's process and takes it back
when it starts again.

The panel also answers [MCP](/docs/mcp) at `/mcp`, so AI agents such as Claude Code or a
bot can read the server and call the tools of your apps.

## What a deploy does

1. oxploy fetches the branch from GitHub, through the GitHub App you connected.
2. It builds the code with `cargo build --release`, or with a build script of your own.
3. It runs the service's script after the build, if it has one, such as a migration.
4. It stops the release that was running, starts the new one on the service's port and
   waits until it listens there.
5. Caddy sends the service's domains to the new release.

If the build or the script fails, the release that was running keeps running. If the new
release exits as it starts, oxploy starts the previous one again. One build runs at a
time on the machine, and the others wait their turn.

## Projects, services and organizations

A **service** is one app: a repository and branch, built into one binary. **Projects**
group services, such as the API and the workers of one product, and projects belong to an
**organization**.

![A project with two services, one running and one not deployed yet](project)

Organizations keep separate sets of projects, with their own [GitHub
accounts](/docs/github) and [MCP tokens](/docs/mcp#organization-tokens). The switcher at
the top of the sidebar changes the one you work in, and makes, renames or deletes them,
with a logo of your own; the default organization is the one the panel opens in, and the
last one cannot be deleted.

The super admin, the first user, owns the server: only they see its settings, its
monitoring and its own [MCP tokens](/docs/mcp#server-tokens).

## Next steps

- [Install oxploy](/docs/installation) on a Linux server.
- [Connect GitHub](/docs/github), then [deploy a service](/docs/deploying).
- [Connect Claude Code](/docs/mcp#connect-claude-code) to the server.
