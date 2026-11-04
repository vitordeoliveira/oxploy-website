oxploy's releases run on Linux on x86_64. The server needs a public address, ports 80
and 443 free for Caddy, and the tools that build Rust apps. These steps use Ubuntu or
Debian, as root or with `sudo`.

## Prepare the server

oxploy builds apps with the server's own `git`, Rust and C compiler. Install them, and
create a user for oxploy, with its home where its data will live:

```sh
sudo apt install -y git build-essential curl
sudo useradd --system --create-home --home-dir /var/lib/oxploy --shell /bin/bash oxploy
```

Install Rust for that user with [rustup](https://rustup.rs), so its builds find it in
`/var/lib/oxploy/.cargo/bin`:

```sh
sudo -u oxploy sh -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
```

Add whatever the crates of your apps link to, such as `pkg-config` and `libssl-dev` for
OpenSSL.

## Install Caddy

Install Caddy from [its packages](https://caddyserver.com/docs/install), which run it as a
service. oxploy loads every route into it, so its Caddyfile only opens the admin API:

```sh
echo '{ admin localhost:2019 }' | sudo tee /etc/caddy/Caddyfile
sudo systemctl restart caddy
```

This Caddy belongs to oxploy, which replaces its whole configuration.

## Install oxploy

Each [release](https://github.com/vitordeoliveira/oxploy/releases) has the binary and the
`assets` folder it loads. Download the latest one, and give the folder to oxploy's user so
that the panel can [update itself](/docs/updating):

```sh
sudo mkdir -p /opt/oxploy
curl -L https://github.com/vitordeoliveira/oxploy/releases/latest/download/oxploy-x86_64-linux.tar.gz | sudo tar xz -C /opt/oxploy
sudo chown -R oxploy: /opt/oxploy
```

## Run it as a service

Write `/etc/systemd/system/oxploy.service`:

```ini
[Unit]
Description=oxploy
After=network-online.target caddy.service
Wants=network-online.target

[Service]
User=oxploy
WorkingDirectory=/var/lib/oxploy
ExecStart=/opt/oxploy/oxploy
Environment=DATABASE_URL=sqlite:/var/lib/oxploy/oxploy.db
Environment=OXPLOY_DATA_DIR=/var/lib/oxploy/data
Environment=PATH=/var/lib/oxploy/.cargo/bin:/usr/local/bin:/usr/bin:/bin
Restart=always
KillMode=process

[Install]
WantedBy=multi-user.target
```

`KillMode=process` keeps the apps running while oxploy stops or restarts, and the `PATH`
is where the builds find cargo. Then start it:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now oxploy
```

## Set it up

On its first run, oxploy writes the address to open and a setup code to its log:

```sh
sudo journalctl -u oxploy | grep "setup code"
```

```text
oxploy: the server has no super admin yet. Open https://203.0.113.7 and enter the setup code K7QM-2XRP-H9TC
```

The code is also in `/var/lib/oxploy/data/setup-code`. Open the address: the panel
answers over HTTPS at the server's public IPv4 address, with a certificate Let's Encrypt
gives the address for six days at a time and Caddy renews. Create the super admin there
with the setup code, which keeps whoever opens the address first from taking the server.

If Let's Encrypt cannot reach the server, Caddy signs a certificate itself, which the
browser warns about. Behind NAT, where the machine does not see its public address, set
`OXPLOY_PUBLIC_IP` in the service.

## Give the panel a domain

The panel can also answer at a domain of its own, which is nicer to remember and is the
address to connect GitHub from:

1. Point the domain's `A` record to the server.
2. In **Settings**, type the domain, such as `panel.example.com`, without `https://`.
3. Wait for the check beside it to say the domain points here, then open it.

The panel keeps answering at the server's address. Each address has its own sign-in, so
sign in again at the domain.

Next, [connect GitHub](/docs/github).
