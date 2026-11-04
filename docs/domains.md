oxploy serves apps at their domains through [Caddy](https://caddyserver.com), which gets
each domain's certificate from Let's Encrypt. You add domains in the service's
**Domains** tab, and oxploy loads every route into Caddy, again whenever Caddy loses
them, as after a restart.

## Add a domain

1. Point the domain's `A` or `AAAA` record to the server. The tab shows the addresses to
   use.
2. Add the domain in the **Domains** tab, such as `shop.example.com`.
3. The check beside it says whether the domain points here, elsewhere, or nowhere yet.

Caddy gets the certificate the first time someone opens the domain, once it points here.
Names under `localhost`, such as `shop.localhost`, get a certificate from Caddy itself,
for trying things on your own machine.

## Options

- **Path**: serve the app under a path only, such as `shop.example.com/api`. Several
  apps can share a domain this way.
- **Strip the path**: with a path, send the app `/orders` rather than `/api/orders`.
- **HTTPS**: on by default. Off, the domain answers over plain HTTP. A domain is either
  HTTPS or HTTP for all its paths.
- **Redirect www**: also answer at `www.shop.example.com`, sending visitors to
  `shop.example.com`.

A domain and path go to one app only, and the panel's own domain is the panel's.

## While the app is not running

A domain whose app was never deployed, or is stopped, answers with a short page that says
so, rather than an error from Caddy.

## An address for every new app

The first time a service goes live, if it has no domain, oxploy gives it one at the
server's public IPv4 address through [sslip.io](https://sslip.io), such as
`shop-api.203-0-113-7.sslip.io`, over HTTPS. It opens right away, without a DNS record.

oxploy gives it only that once, so a domain you delete stays deleted. Let's Encrypt's
limits for sslip.io are shared by everyone who uses it; if a certificate does not come,
switch the domain to plain HTTP or add one of your own.

## When Caddy is not running

The **Domains** tab and **Settings** say when oxploy cannot reach Caddy, or when Caddy
refused the routes, with what went wrong. Caddy's admin API is at `127.0.0.1:2019` unless
`OXPLOY_CADDY_ADMIN` says otherwise; see [Configuration](/docs/configuration).
