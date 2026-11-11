Once a day, oxploy tells oxploy.com that it runs, with its version. That count is how the
project knows how many servers run oxploy, which versions they are on, and whether people
keep it after they install it. It says nothing about you, your apps or your server, and
you can turn it off.

## What it sends

A report is a small JSON, sent by `POST` to `https://oxploy.com/api/ping`:

```json
{
  "install_id": "5f0c3a9e1b7d4c2a8e6f0b1d3c5a7e9f",
  "version": "0.10.0",
  "platform": "x86_64-linux"
}
```

- `install_id` is a random number oxploy made the first time it started. It is not made
  from anything, such as the server's address, its domain or your email, so it only tells
  the reports of one server apart.
- `version` is the version of oxploy that runs.
- `platform` is the machine it runs on.

**Settings** shows the report as oxploy would send it now, and when it last sent one.

## Share what it runs

If you choose to share it, in **Settings**, the report also says what the server runs, in
numbers only:

```json
"usage": {
  "users": 2,
  "organizations": 1,
  "apps": 4,
  "apps_built_with_cargo": 3,
  "apps_built_with_script": 0,
  "apps_deploying_releases": 1,
  "apps_with_mcp": 2,
  "external_mcp_servers": 1,
  "mcp_tokens": 3,
  "databases": 2,
  "domains": 3,
  "deployments_7d": 9,
  "failed_deployments_7d": 1
}
```

They tell which parts of oxploy are used, and how often deployments fail, which is where
the work goes next. It is off until you turn it on.

## What it never sends

No names, of users, organizations, projects or apps. No domains, repositories, commits,
environment variables, logs, tokens or addresses. Nothing about the apps' visitors.

## What the site keeps

The site keeps the `install_id` only as a code made from it with a secret, with the days
each server reported in, its latest version, machine and numbers. It does not keep the
address the report came from. A server not heard from in 400 days is deleted. The
[privacy page](/privacy) says the same.

## Turn it off

In **Settings**, turn **Report that this server runs** off, and oxploy sends nothing. Or set
one of these in its [environment](/docs/configuration), which turns it off whatever
Settings says:

```ini
Environment=OXPLOY_TELEMETRY=off
```

`DO_NOT_TRACK=1` turns it off too. A build of oxploy made with `cargo build` without
`--release`, as in development, does not report either.
