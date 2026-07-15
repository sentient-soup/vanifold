#!/usr/bin/env bash
# Install or update a vanifold hub on Debian-family Linux (Raspberry Pi OS).
# Sets up the Mosquitto broker, installs the vanifold-core binary (UI
# embedded), a systemd service, and runs an end-to-end smoke test.
#
# Install/update from the latest GitHub release (no toolchain needed):
#   curl -fsSL https://raw.githubusercontent.com/sentient-soup/vanifold/main/scripts/install.sh | sudo bash
#
# Pin a version:
#   ... | sudo VANIFOLD_VERSION=v0.1.0 bash
#
# Build from source instead (the one-stop dev loop: pulls, rebuilds whatever
# changed, reinstalls, restarts):
#   sudo ./scripts/install.sh --source
#
# Idempotent: safe to re-run; existing passwords and data are kept.
set -euo pipefail

REPO="sentient-soup/vanifold"
SRC_DIR="/opt/vanifold/src"
CONF_DIR="/etc/vanifold"
DATA_DIR="/var/lib/vanifold"
BIN=/usr/local/bin/vanifold-core
API_PORT=8480

FROM_SOURCE=0
[ "${1:-}" = "--source" ] && FROM_SOURCE=1

[ "$(id -u)" -eq 0 ] || { echo "run as root (sudo)"; exit 1; }

echo "==> [1/5] apt packages"
apt-get update -qq
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq curl mosquitto mosquitto-clients

echo "==> [2/5] MQTT credentials + broker config"
# Two principals per docs/mqtt-conventions.md: the core, and one shared
# device/node identity. Passwords generated once, kept on re-run.
mkdir -p "$CONF_DIR"
chmod 750 "$CONF_DIR"
if [ ! -f "$CONF_DIR/mqtt-credentials" ]; then
    CORE_PASS=$(head -c16 /dev/urandom | od -An -tx1 | tr -d ' \n')
    NODE_PASS=$(head -c16 /dev/urandom | od -An -tx1 | tr -d ' \n')
    cat > "$CONF_DIR/mqtt-credentials" <<EOF
CORE_USER=vanifold-core
CORE_PASS=$CORE_PASS
NODE_USER=vanifold-node
NODE_PASS=$NODE_PASS
EOF
    chmod 600 "$CONF_DIR/mqtt-credentials"
fi
# shellcheck source=/dev/null
. "$CONF_DIR/mqtt-credentials"

touch /etc/mosquitto/passwd
mosquitto_passwd -b /etc/mosquitto/passwd "$CORE_USER" "$CORE_PASS"
mosquitto_passwd -b /etc/mosquitto/passwd "$NODE_USER" "$NODE_PASS"
chown mosquitto:mosquitto /etc/mosquitto/passwd
chmod 600 /etc/mosquitto/passwd

cat > /etc/mosquitto/conf.d/vanifold.conf <<'EOF'
# vanifold broker config (docs/mqtt-conventions.md): van-LAN only, no TLS,
# credentials required. Remote access is a VPN problem, not a broker problem.
listener 1883
allow_anonymous false
password_file /etc/mosquitto/passwd
EOF
systemctl enable --now mosquitto
systemctl restart mosquitto

if [ "$FROM_SOURCE" = 1 ]; then
    echo "==> [3/5] Build from source"
    command -v git >/dev/null || apt-get install -y -qq git build-essential pkg-config
    export PATH="$HOME/.cargo/bin:$PATH"
    if ! command -v cargo >/dev/null; then
        echo "    installing Rust toolchain..."
        curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal -q
    fi
    if ! command -v npm >/dev/null; then
        echo "node/npm not found. Install Node 20+ first, e.g.:" >&2
        echo "  curl -fsSL https://deb.nodesource.com/setup_22.x | sudo bash - && sudo apt-get install -y nodejs" >&2
        exit 1
    fi
    if [ -d "$SRC_DIR/.git" ]; then
        git -C "$SRC_DIR" pull --ff-only
    else
        mkdir -p "$(dirname "$SRC_DIR")"
        git clone --depth 1 "https://github.com/$REPO.git" "$SRC_DIR"
    fi
    npm install --prefix "$SRC_DIR/ui" --no-fund --no-audit
    npm run build --prefix "$SRC_DIR/ui"
    cargo build --release -p vanifold-core --features embed-ui \
        --manifest-path "$SRC_DIR/Cargo.toml"
    install -m 755 "$SRC_DIR/target/release/vanifold-core" "$BIN"
else
    echo "==> [3/5] Install release binary"
    case "$(uname -m)" in
        aarch64) TARGET=aarch64-unknown-linux-gnu ;;
        armv7l)  TARGET=armv7-unknown-linux-gnueabihf ;;
        x86_64)  TARGET=x86_64-unknown-linux-gnu ;;
        *) echo "unsupported architecture $(uname -m); try --source" >&2; exit 1 ;;
    esac
    VERSION="${VANIFOLD_VERSION:-}"
    if [ -z "$VERSION" ]; then
        VERSION=$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
            "https://github.com/$REPO/releases/latest" | sed 's#.*/tag/##')
    fi
    if [ -z "$VERSION" ] || [ "${VERSION#v}" = "$VERSION" ]; then
        echo "could not resolve a release version (no releases yet?); try --source" >&2
        exit 1
    fi
    NAME="vanifold-$VERSION-$TARGET"
    URL="https://github.com/$REPO/releases/download/$VERSION/$NAME.tar.gz"
    TMP=$(mktemp -d)
    trap 'rm -rf "$TMP"' EXIT
    echo "    fetching $NAME ..."
    curl -fsSL -o "$TMP/$NAME.tar.gz" "$URL"
    # the release publishes one combined SHA256SUMS, not per-file sidecars
    curl -fsSL -o "$TMP/SHA256SUMS" \
        "https://github.com/$REPO/releases/download/$VERSION/SHA256SUMS"
    (cd "$TMP" && grep "  $NAME.tar.gz\$" SHA256SUMS | sha256sum -c - >/dev/null)
    tar -xzf "$TMP/$NAME.tar.gz" -C "$TMP"
    install -m 755 "$TMP/$NAME/vanifold-core" "$BIN"
    echo "    installed $VERSION"
fi

echo "==> [4/5] vanifold service"
id -u vanifold >/dev/null 2>&1 || useradd --system --home "$DATA_DIR" --shell /usr/sbin/nologin vanifold
mkdir -p "$DATA_DIR"
chown vanifold:vanifold "$DATA_DIR"
chgrp vanifold "$CONF_DIR"

cat > "$CONF_DIR/vanifold.toml" <<EOF
stale_after_secs = 900

[mqtt]
host = "localhost"
port = 1883
username = "$CORE_USER"
password = "$CORE_PASS"
client_id = "vanifold-core"

[api]
listen = "0.0.0.0:$API_PORT"

[store]
db_path = "$DATA_DIR/vanifold.db"
EOF
chown root:vanifold "$CONF_DIR/vanifold.toml"
chmod 640 "$CONF_DIR/vanifold.toml"

cat > /etc/systemd/system/vanifold.service <<EOF
[Unit]
Description=Vanifold hub daemon
After=network-online.target mosquitto.service
Wants=mosquitto.service

[Service]
User=vanifold
Group=vanifold
ExecStart=$BIN $CONF_DIR/vanifold.toml
WorkingDirectory=$DATA_DIR
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable vanifold
systemctl restart vanifold

echo "==> [5/5] Smoke test: fake device announces itself over HA discovery"
sleep 3
TEST_TOPIC="homeassistant/sensor/bootstrap-test/config"
mosquitto_pub -u "$NODE_USER" -P "$NODE_PASS" -r -t "$TEST_TOPIC" \
    -m '{"uniq_id":"bootstrap-test","name":"Bootstrap Test","stat_t":"bootstrap/temp","unit_of_meas":"C","dev":{"ids":["bootstrap-rig"],"name":"Bootstrap Rig"}}'
mosquitto_pub -u "$NODE_USER" -P "$NODE_PASS" -t "bootstrap/temp" -m "21.5"
sleep 2
BODY=$(curl -fsS "http://localhost:$API_PORT/api/entities" || true)
# Clean up the fake device either way (empty retained payload = removal).
mosquitto_pub -u "$NODE_USER" -P "$NODE_PASS" -r -t "$TEST_TOPIC" -m ''

if echo "$BODY" | grep -q '"bootstrap-test"' && echo "$BODY" | grep -q '21.5'; then
    echo ""
    echo "SMOKE TEST PASSED: discovery -> registry -> API pipeline is live."
else
    echo ""
    echo "SMOKE TEST FAILED. Inspect with:"
    echo "  journalctl -u vanifold -n 50"
    echo "  systemctl status mosquitto vanifold"
    exit 1
fi

IP=$(hostname -I | awk '{print $1}')
cat <<EOF

vanifold hub is up.

  Dashboard:  http://$IP:$API_PORT/
  API:        http://$IP:$API_PORT/api/entities
  Logs:       journalctl -u vanifold -f

Device MQTT credentials (for ESPHome/Shelly/zigbee2mqtt configs):
  broker:   $IP:1883
  username: $NODE_USER
  password: $NODE_PASS

To update: re-run this script.
EOF
