oxploy's releases run on Linux on x86_64. The server needs a public address, ports 80
and 443 free for Caddy, and the tools that build Rust apps. On a Mac, oxploy runs from its
source code: see [On macOS](#on-macos).

## Install with one command

On Debian or Ubuntu, as root:

```sh
curl -sSL https://oxploy.com/install.sh | sh
```

Or, as a user who can use `sudo`, `curl -sSL https://oxploy.com/install.sh | sudo sh`. The
[script](https://oxploy.com/install.sh) does what the steps below do by hand:

1. Installs `git`, a C compiler, `pkg-config` and `libssl-dev`, which crates that use
   OpenSSL link to.
2. Creates the `oxploy` user, with its home in `/var/lib/oxploy`, and installs Rust for
   it with rustup.
3. Installs Caddy from its packages, with the Caddyfile oxploy needs. It keeps the
   Caddyfile there was in `/etc/caddy/Caddyfile.before-oxploy`.
4. Downloads oxploy's latest release to `/opt/oxploy`, checked against its checksum.
5. Runs oxploy as a systemd service, opens ports 80 and 443 when ufw guards the server,
   and prints the address to open and the [setup code](#set-it-up).

It stops before changing anything when the server is not one it knows, or when another
program, such as nginx, holds port 80 or 443. Running it again keeps what is there and
adds what is missing; once installed, oxploy [updates itself](/docs/updating) from its
panel.

## Install by hand

These steps use Ubuntu or Debian, as root or with `sudo`.

### Prepare the server

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

### Install Caddy

Install Caddy from [its packages](https://caddyserver.com/docs/install), which run it as a
service. oxploy loads every route into it, so its Caddyfile only opens the admin API:

```sh
printf '{\n\tadmin localhost:2019\n}\n' | sudo tee /etc/caddy/Caddyfile
sudo systemctl restart caddy
```

This Caddy belongs to oxploy, which replaces its whole configuration.

### Install oxploy

Each [release](https://github.com/vitordeoliveira/oxploy/releases) has the binary and the
`assets` folder it loads. Download the latest one, and give the folder to oxploy's user so
that the panel can [update itself](/docs/updating):

```sh
sudo mkdir -p /opt/oxploy
curl -L https://github.com/vitordeoliveira/oxploy/releases/latest/download/oxploy-x86_64-linux.tar.gz | sudo tar xz -C /opt/oxploy
sudo chown -R oxploy: /opt/oxploy
```

### Run it as a service

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

### Docker apps

To also run [Docker apps](/docs/deploying#docker-apps), install
[Docker](https://docs.docker.com/engine/install/) and add oxploy's user to the `docker`
group, then restart oxploy:

```sh
sudo usermod -aG docker oxploy
sudo systemctl restart oxploy
```

The `docker` group lets oxploy do anything on the server, as root can, so add it only if
you run Docker apps. Rust apps need neither.

## On macOS

oxploy's code builds on macOS too, but its releases are for Linux only, so on a Mac, such
as to try it on your own machine, it runs from its [source
code](https://github.com/vitordeoliveira/oxploy). It needs the Xcode command line tools,
Rust from [rustup](https://rustup.rs), the Topcoat CLI, and Caddy from Homebrew:

```sh
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install topcoat-cli
brew install caddy
```

Caddy runs as a Homebrew service, with the Caddyfile oxploy needs:

```sh
printf '{\n\tadmin localhost:2019\n}\n' > "$(brew --prefix)/etc/Caddyfile"
brew services start caddy
```

Then get the code, build it with its assets, and start it:

```sh
git clone https://github.com/vitordeoliveira/oxploy
cd oxploy
cargo build --release
topcoat asset bundle --release
./target/release/oxploy
```

It keeps its database in `data.db` and the apps in `data/`, in the folder it starts from;
`DATABASE_URL` and `OXPLOY_DATA_DIR` put them elsewhere, as in
[Configuration](/docs/configuration). Open `http://127.0.0.1:3000` and create the super
admin with the setup code it prints, as [below](#set-it-up). Without HTTPS, signing in
works only at `127.0.0.1` or `localhost`.

oxploy runs while its terminal does; the apps keep running when it stops, and it takes
them back when it starts again. A Mac has no public address, so a new app gets no
[sslip.io domain](/docs/domains#an-address-for-every-new-app); names under `localhost`,
such as `shop.localhost`, work for trying. The apps build with cargo on the Mac, and a
[GitHub release](/docs/deploying#deploy-github-releases) must have a file built for it:
a Linux binary fails its deployment.

The panel says when a new release is out, but has no build of it for a Mac. To update,
`git pull` in the folder and run the build commands again.

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

![The first-run page, which creates the super admin with the setup code](setup)

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

![The Panel domain and Time zone cards in Settings](settings)

## Pick the time zone

The panel shows times in the server's time zone, UTC on most servers. **Settings** picks
another one for the panel: its clock, its charts and dates, and the times oxploy writes in
the apps' logs and backups. The apps keep the server's own.

Next, [connect GitHub](/docs/github).
