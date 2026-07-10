#!/usr/bin/env bash
# Bootstrap a headless Raspberry Pi OS (Debian) machine into a vanifold hub:
# Mosquitto broker with credentials, vanifold-core built from source, systemd
# service, and an end-to-end discovery smoke test.
#
# Usage:  curl -fsSL https://raw.githubusercontent.com/sentient-soup/vanifold/main/scripts/bootstrap-pi.sh | sudo bash
#    or:  sudo ./bootstrap-pi.sh
#
# Idempotent: safe to re-run; existing passwords and data are kept.
set -euo pipefail

REPO_URL="https://github.com/sentient-soup/vanifold.git"
SRC_DIR="/opt/vanifold/src"
CONF_DIR="/etc/vanifold"
DATA_DIR="/var/lib/vanifold"
API_PORT=8480

[ "$(id -u)" -eq 0 ] || { echo "run as root (sudo)"; exit 1; }

echo "==> [1/6] apt packages (build tools, mosquitto)"
apt-get update -qq
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    git build-essential pkg-config curl mosquitto mosquitto-clients

echo "==> [2/6] MQTT credentials + broker config"
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

echo "==> [3/6] Rust toolchain (skipped if present)"
if ! command -v cargo >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/cargo" ]; then
    curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal -q
fi
export PATH="$HOME/.cargo/bin:$PATH"

echo "==> [4/6] Build vanifold-core (first build takes 10-20 min on a Pi 4)"
mkdir -p "$(dirname "$SRC_DIR")"
if [ -d "$SRC_DIR/.git" ]; then
    git -C "$SRC_DIR" pull --ff-only
else
    git clone --depth 1 "$REPO_URL" "$SRC_DIR"
fi
cargo build --release --manifest-path "$SRC_DIR/core/Cargo.toml"
install -m 755 "$SRC_DIR/core/target/release/vanifold-core" /usr/local/bin/vanifold-core

echo "==> [5/6] vanifold service"
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
ExecStart=/usr/local/bin/vanifold-core $CONF_DIR/vanifold.toml
WorkingDirectory=$DATA_DIR
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable vanifold
systemctl restart vanifold

echo "==> [6/6] Smoke test: fake device announces itself over HA discovery"
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

  API:        http://$IP:$API_PORT/api/entities
  Live WS:    ws://$IP:$API_PORT/api/ws
  Logs:       journalctl -u vanifold -f

Device MQTT credentials (for ESPHome/Shelly/zigbee2mqtt configs):
  broker:   $IP:1883
  username: $NODE_USER
  password: $NODE_PASS

To update later: re-run this script (it pulls and rebuilds).
EOF
