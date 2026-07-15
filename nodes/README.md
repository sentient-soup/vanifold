# nodes/

ESPHome configurations for vanifold nodes: distributed processors placed near each
subsystem cluster. Per the three-tier law (docs/DESIGN.md), control loops and
failsafes run ON the node; the hub only orchestrates.

Nodes:
- `electronics-bay.yaml`: DS18B20 battery temps, 2ch relay, PWM light strip
- `rear-entry` (planned): winch door as a feedback cover (interlocked solenoid
  relays, limit switches, current-sense stall cutoff, max-runtime timeout)

## Flashing

```sh
cp secrets.yaml.example secrets.yaml   # fill in wifi + broker + passwords
uvx esphome run electronics-bay.yaml   # first flash over USB, then OTA
```

First boot logs print DS18B20 1-Wire addresses; paste them into the YAML's
`dallas_temp` sensors and reflash. Convention (docs/mqtt-conventions.md):
MQTT mode with no `api:` block, discovery on, hostname `vanifold-<node>`.
