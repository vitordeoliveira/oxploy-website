#!/bin/sh
# Installs oxploy on a Debian or Ubuntu server on x86_64, as root:
#
#   curl -sSL https://oxploy.com/install.sh | sh
#
# It does what https://oxploy.com/docs/installation does by hand: the tools that build
# Rust apps, a user for oxploy with Rust, Caddy, and oxploy itself as a systemd service,
# then prints the address to open and the setup code. Running it again keeps what is
# there and adds what is missing; oxploy updates itself from its panel.
set -eu

RELEASE=https://github.com/vitordeoliveira/oxploy/releases/latest/download/oxploy-x86_64-linux.tar.gz
INSTALL_DIR=/opt/oxploy
OXPLOY_HOME=/var/lib/oxploy
OXPLOY_USER=oxploy
SERVICE=/etc/systemd/system/oxploy.service
CADDYFILE=/etc/caddy/Caddyfile
# The Caddyfile oxploy needs: its admin API only, since oxploy loads every route. Caddy
# takes a block's braces only at the end and the start of their lines.
CADDY_CONFIG='{
	admin localhost:2019
}'

# Bold and red only in a terminal.
if [ -t 1 ]; then
	BOLD=$(printf '\033[1m') RED=$(printf '\033[31m') PLAIN=$(printf '\033[0m')
else
	BOLD='' RED='' PLAIN=''
fi

say() {
	printf '%s==> %s%s\n' "$BOLD" "$*" "$PLAIN"
}

fail() {
	printf '%soxploy: %s%s\n' "$RED" "$*" "$PLAIN" >&2
	exit 1
}

# What the server must be, before anything changes on it.
check() {
	[ "$(uname -s)" = Linux ] && [ "$(uname -m)" = x86_64 ] ||
		fail "oxploy's releases run on Linux on x86_64, and this is $(uname -s) on $(uname -m)."
	[ "$(id -u)" = 0 ] || fail "run it as root, such as: curl -sSL https://oxploy.com/install.sh | sudo sh"
	command -v apt-get >/dev/null 2>&1 ||
		fail "this script installs with apt, on Debian or Ubuntu. Elsewhere, follow https://oxploy.com/docs/installation"
	[ -d /run/systemd/system ] || fail "oxploy runs as a systemd service, and systemd does not run here."
	# Caddy, oxploy's own, answers at 80 and 443.
	if command -v ss >/dev/null 2>&1; then
		for port in 80 443; do
			taken=$(ss -Hltnp "sport = :$port" 2>/dev/null | grep -v '"caddy"' || true)
			[ -z "$taken" ] || fail "port $port is taken by another program, and oxploy's Caddy needs it: $taken"
		done
	fi
}

# git, a C compiler and what most crates link to, such as OpenSSL.
install_tools() {
	say "Installing git, a C compiler and OpenSSL's headers"
	export DEBIAN_FRONTEND=noninteractive
	apt-get update -q
	apt-get install -y -q git build-essential pkg-config libssl-dev curl ca-certificates gnupg
}

# A user for oxploy, whose home keeps its data, and Rust in its home for the builds.
install_user() {
	if id "$OXPLOY_USER" >/dev/null 2>&1; then
		say "The $OXPLOY_USER user is there"
	else
		say "Creating the $OXPLOY_USER user, with its home at $OXPLOY_HOME"
		useradd --system --create-home --home-dir "$OXPLOY_HOME" --shell /bin/bash "$OXPLOY_USER"
	fi
	if [ -x "$OXPLOY_HOME/.cargo/bin/cargo" ]; then
		say "Rust is there"
	else
		say "Installing Rust with rustup, for the $OXPLOY_USER user"
		su - "$OXPLOY_USER" -s /bin/sh -c \
			"curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal"
	fi
}

# Caddy from its own packages, which run it as a service, with the Caddyfile oxploy needs.
install_caddy() {
	if command -v caddy >/dev/null 2>&1; then
		say "Caddy is there"
	else
		say "Installing Caddy from its packages"
		curl -1sLf https://dl.cloudsmith.io/public/caddy/stable/gpg.key |
			gpg --dearmor --yes -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
		curl -1sLf https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt >/etc/apt/sources.list.d/caddy-stable.list
		chmod o+r /usr/share/keyrings/caddy-stable-archive-keyring.gpg /etc/apt/sources.list.d/caddy-stable.list
		apt-get update -q
		apt-get install -y -q caddy
	fi
	if [ "$(cat "$CADDYFILE" 2>/dev/null)" != "$CADDY_CONFIG" ]; then
		# oxploy replaces Caddy's whole configuration, so the one before is kept aside.
		if [ -f "$CADDYFILE" ] && [ ! -f "$CADDYFILE.before-oxploy" ]; then
			cp "$CADDYFILE" "$CADDYFILE.before-oxploy"
			say "Kept Caddy's Caddyfile in $CADDYFILE.before-oxploy: this Caddy is oxploy's now"
		fi
		mkdir -p "$(dirname "$CADDYFILE")"
		printf '%s\n' "$CADDY_CONFIG" >"$CADDYFILE"
		systemctl restart caddy || fail "Caddy did not start. See why with: journalctl -u caddy"
	fi
	# A Caddy already set up keeps running, and the apps behind it with it.
	systemctl enable caddy >/dev/null 2>&1
	systemctl start caddy || fail "Caddy did not start. See why with: journalctl -u caddy"
}

# The latest release, checked against its checksum, in a folder oxploy's user owns, so
# that the panel can update itself.
install_oxploy() {
	if [ -x "$INSTALL_DIR/oxploy" ]; then
		say "oxploy is there: its panel updates it"
		return
	fi
	fresh=1
	say "Downloading oxploy's latest release"
	download=$(mktemp -d)
	curl -fsSL --retry 3 -o "$download/oxploy.tar.gz" "$RELEASE"
	curl -fsSL --retry 3 -o "$download/oxploy.tar.gz.sha256" "$RELEASE.sha256"
	expected=$(cut -d ' ' -f 1 "$download/oxploy.tar.gz.sha256")
	actual=$(sha256sum "$download/oxploy.tar.gz" | cut -d ' ' -f 1)
	[ "$expected" = "$actual" ] || fail "the download does not match its checksum. Try again."
	mkdir -p "$INSTALL_DIR"
	tar xzf "$download/oxploy.tar.gz" -C "$INSTALL_DIR"
	rm -rf "$download"
	chown -R "$OXPLOY_USER:" "$INSTALL_DIR"
}

# oxploy as a service, whose apps keep running while it stops, restarts or updates.
install_service() {
	if [ ! -f "$SERVICE" ]; then
		say "Running oxploy as a systemd service"
		cat >"$SERVICE" <<EOF
[Unit]
Description=oxploy
After=network-online.target caddy.service
Wants=network-online.target

[Service]
User=$OXPLOY_USER
WorkingDirectory=$OXPLOY_HOME
ExecStart=$INSTALL_DIR/oxploy
Environment=DATABASE_URL=sqlite:$OXPLOY_HOME/oxploy.db
Environment=OXPLOY_DATA_DIR=$OXPLOY_HOME/data
Environment=PATH=$OXPLOY_HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin
Restart=always
KillMode=process

[Install]
WantedBy=multi-user.target
EOF
		systemctl daemon-reload
	fi
	systemctl enable --now oxploy >/dev/null 2>&1
}

# The firewall, when ufw runs one, lets Caddy's ports through.
open_ports() {
	if command -v ufw >/dev/null 2>&1 && ufw status 2>/dev/null | grep -q '^Status: active'; then
		say "Opening ports 80 and 443 in ufw"
		ufw allow 80/tcp >/dev/null
		ufw allow 443/tcp >/dev/null
	fi
}

# The address to open and the setup code, which oxploy writes to its log and keeps in
# its data folder until the super admin exists.
finish() {
	code="$OXPLOY_HOME/data/setup-code"
	if [ "$fresh" = 1 ]; then
		say "Waiting for oxploy to start"
		for _ in $(seq 1 60); do
			[ -s "$code" ] && break
			sleep 1
		done
		# The log line comes right after the file.
		sleep 1
	fi
	printf '\n'
	if [ -s "$code" ]; then
		line=$(journalctl -u oxploy --no-pager -o cat 2>/dev/null | grep 'setup code' | tail -n 1 || true)
		case "$line" in
		*Open*) printf '%sOpen %s%s\n' "$BOLD" "${line#*Open }" "$PLAIN" ;;
		*) printf '%sOpen https:// and the server'"'"'s address, and enter the setup code %s%s\n' "$BOLD" "$(cat "$code")" "$PLAIN" ;;
		esac
		printf 'The code is also in %s.\n' "$code"
	elif systemctl is-active --quiet oxploy; then
		printf 'oxploy is running, and has its super admin. Open the panel at its domain, or at https:// and the server'"'"'s address.\n'
	else
		fail "oxploy did not start. See why with: journalctl -u oxploy"
	fi
	printf 'Behind NAT, set OXPLOY_PUBLIC_IP in %s. Next: https://oxploy.com/docs/github\n' "$SERVICE"
	printf 'oxploy tells oxploy.com once a day that it runs, with its version and a random ID. See what it sends, or turn it off in Settings: https://oxploy.com/docs/usage-reporting\n'
}

main() {
	# Whether this run installed oxploy, which then starts for the first time.
	fresh=0
	check
	install_tools
	install_user
	install_caddy
	install_oxploy
	install_service
	open_ports
	finish
}

# All of it runs once the whole script has arrived, so a cut download runs nothing.
main "$@"
