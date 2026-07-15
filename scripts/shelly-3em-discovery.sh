#!/bin/bash
# Announce a Gen1 Shelly 3EM to vanifold via HA MQTT Discovery.
#
# Gen1 Shelly firmware publishes plain `shellies/<id>/...` state topics but no
# `homeassistant/.../config` announcements, so this script publishes retained
# discovery configs on the device's behalf, pointing at its real topics.
# Run once per device (configs are retained); re-running is idempotent.
#
# Usage: shelly-3em-discovery.sh <device-id> [mosquitto_pub args...]
#   device-id  the Shelly's MQTT id, e.g. shellyem3-C45BBE. Find it in the
#              Shelly web UI (Settings > Device Info) or watch
#              `mosquitto_sub -t shellies/announce` and power-cycle the device.
#   extras     passed through to mosquitto_pub, e.g. -h <broker> -u <user> -P <pass>
#
# Example (on the hub Pi):
#   ./shelly-3em-discovery.sh shellyem3-C45BBE -u vanifold-node -P "$PASS"
set -euo pipefail

ID=${1:?usage: $0 <device-id> [mosquitto_pub args...]}
shift
EXTRA=("$@")

BASE="shellies/$ID"
DEVICE='{"identifiers":["'"$ID"'"],"name":"Shelly 3EM","manufacturer":"Shelly","model":"SHEM-3"}'
AVAIL='"availability_topic":"'"$BASE"'/online","payload_available":"true","payload_not_available":"false"'

pub() { # <component> <object_id>, payload on stdin
  mosquitto_pub "${EXTRA[@]}" -r -q 1 -t "homeassistant/$1/$ID/$2/config" -s
}

PHASES=(A B C)
for i in 0 1 2; do
  P=${PHASES[$i]}
  # ponytail: power/voltage/current/energy only; pf and total_returned exist
  # on the same topic tree, add if a panel ever wants them
  pub sensor "emeter-$i-power" <<EOF
{"unique_id":"$ID-emeter-$i-power","name":"Phase $P power",
 "state_topic":"$BASE/emeter/$i/power","unit_of_measurement":"W",
 "device_class":"power",$AVAIL,"device":$DEVICE}
EOF
  pub sensor "emeter-$i-voltage" <<EOF
{"unique_id":"$ID-emeter-$i-voltage","name":"Phase $P voltage",
 "state_topic":"$BASE/emeter/$i/voltage","unit_of_measurement":"V",
 "device_class":"voltage",$AVAIL,"device":$DEVICE}
EOF
  pub sensor "emeter-$i-current" <<EOF
{"unique_id":"$ID-emeter-$i-current","name":"Phase $P current",
 "state_topic":"$BASE/emeter/$i/current","unit_of_measurement":"A",
 "device_class":"current",$AVAIL,"device":$DEVICE}
EOF
  pub sensor "emeter-$i-total" <<EOF
{"unique_id":"$ID-emeter-$i-total","name":"Phase $P energy",
 "state_topic":"$BASE/emeter/$i/total","unit_of_measurement":"Wh",
 "device_class":"energy","state_class":"total_increasing",$AVAIL,"device":$DEVICE}
EOF
done

pub switch "relay-0" <<EOF
{"unique_id":"$ID-relay-0","name":"Relay",
 "state_topic":"$BASE/relay/0","command_topic":"$BASE/relay/0/command",
 "payload_on":"on","payload_off":"off",$AVAIL,"device":$DEVICE}
EOF

echo "Published 13 retained discovery configs for $ID"
