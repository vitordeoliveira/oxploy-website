#!/usr/bin/env bash
# Takes the docs' screenshots of a demo panel with only example data, writes them to
# assets/docs as WebP, and their sizes to src/features/docs/pictures.rs. See README.md.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
site=$(cd "$here/../.." && pwd)
oxploy=$(cd "${OXPLOY_DIR:-$site/../oxploy}" && pwd)
out=${OUT_DIR:-$site/assets/docs}
panel_port=${PANEL_PORT:-3950}
fresh_port=${FRESH_PORT:-3951}
app_port=${APP_PORT:-10050}
monitor_wait=${MONITOR_WAIT:-180}

for tool in cargo topcoat node npm sqlite3 cwebp curl lsof; do
  command -v "$tool" >/dev/null || { echo "Install $tool first." >&2; exit 1; }
done
for port in "$panel_port" "$fresh_port" "$app_port"; do
  if lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null; then
    echo "Port $port is taken: set PANEL_PORT, FRESH_PORT or APP_PORT to free ones." >&2
    exit 1
  fi
done

# Folders and profiles of this run, by their real path, as the panel shows it.
work=$(cd "$(mktemp -d)" && pwd -P)
pids=()
stop() {
  # The demo app runs apart from the panel, as oxploy's apps do.
  local app_pid="$work/demo/data/apps/shop-docs/app.pid"
  [ -s "$app_pid" ] && kill "$(head -n 1 "$app_pid")" 2>/dev/null || true
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
}
trap stop EXIT

wait_for() { # what, command...
  local what=$1
  shift
  for _ in $(seq 1 120); do
    "$@" >/dev/null 2>&1 && return 0
    sleep 0.5
  done
  echo "Gave up waiting for $what." >&2
  exit 1
}

# A panel of its own, with its database and data in $work/<name>. Caddy's admin API is
# nowhere, so nothing touches this machine's Caddy, and the public address is one of
# the documentation's. The debug build needs a deeper stack than threads get by default.
start_panel() { # name port
  local dir="$work/$1"
  mkdir -p "$dir/data"
  (cd "$dir" && exec env TZ=UTC RUST_MIN_STACK=33554432 \
    DATABASE_URL="sqlite:$dir/oxploy.db" OXPLOY_DATA_DIR="$dir/data" HOST=127.0.0.1 PORT="$2" \
    OXPLOY_CADDY_ADMIN=127.0.0.1:1 OXPLOY_CADDY_LOGS="127.0.0.1:$(($2 + 10000))" \
    OXPLOY_PUBLIC_IP=203.0.113.7 "$oxploy/target/debug/oxploy" >"$dir/oxploy.log" 2>&1) &
  pids+=($!)
  wait_for "the $1 panel" curl -sf -o /dev/null "http://127.0.0.1:$2/register"
  wait_for "the $1 setup code" test -s "$dir/data/setup-code"
}

echo "==> Building oxploy and this site"
(cd "$oxploy" && cargo build --quiet && topcoat asset bundle >/dev/null)
(cd "$site" && cargo build --quiet && topcoat asset bundle >/dev/null)
[ -d "$here/node_modules" ] || (cd "$here" && npm ci --silent)

export PANEL_URL="http://127.0.0.1:$panel_port"
export PROFILE="$work/profile"

echo "==> Seeding a demo panel at $PANEL_URL"
start_panel demo "$panel_port"
node "$here/seed.mjs" start "$(tr -d '[:space:]' <"$work/demo/data/setup-code")"
db="$work/demo/oxploy.db"
sqlite3 "$db" "update organizations set name = 'Acme' where id = 1"

# The docs service runs this site's build as if it had deployed it: a release of its own,
# which the panel starts, as it does every service with a live release.
release="$work/demo/data/apps/shop-docs/releases/1"
mkdir -p "$release/bin" "$release/source"
cp "$site/target/debug/oxploy-website" "$release/bin/"
cp -R "$site/target/debug/assets" "$release/bin/"
now=$(date +%s)
sqlite3 "$db" "insert into deployments (service_id, status, \"commit\", commit_message, binary, created_at, finished_at)
  values (2, 'succeeded', 'a3f9c21e7d4b8f6052c1e9a7b3d8f4c6e2a1b5d9', 'Serve the documentation over MCP', 'oxploy-website', $((now - 300)), $((now - 240)));
  update services set release_id = last_insert_rowid(), port = $app_port, build_kind = 'cargo' where id = 2;"
wait_for "the docs app" curl -s -o /dev/null "http://127.0.0.1:$app_port/"
node "$here/seed.mjs" app

echo "==> Letting the charts fill for ${monitor_wait}s"
sleep "$monitor_wait"

echo "==> Taking the screenshots"
export SHOTS_DIR="$work/shots"
mkdir -p "$SHOTS_DIR"
DEMO_DATA="$work/demo/data" node "$here/capture.mjs"

start_panel fresh "$fresh_port"
FRESH_URL="http://127.0.0.1:$fresh_port" FRESH_DATA="$work/fresh/data" PROFILE="$work/profile-fresh" \
  node "$here/setup.mjs"

echo "==> Writing them to $out"
mkdir -p "$out"
for png in "$SHOTS_DIR"/*.png; do
  cwebp -quiet -q 86 -m 6 "$png" -o "$out/$(basename "$png" .png).webp"
done
node "$here/sizes.mjs" "$SHOTS_DIR" "$site/src/features/docs/pictures.rs"
echo "==> Done. Look at each one before you commit it: none may show this machine."
