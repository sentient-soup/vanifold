# nodes/

ESPHome configurations for vanifold nodes: distributed processors placed near each
subsystem cluster. Per the three-tier law (docs/DESIGN.md), control loops and
failsafes run ON the node; the hub only orchestrates.

Nodes:
- `electronics-bay.yaml`: DS18B20 temps, pixel strip; currently doubling as
  the winch-door bench sim (one reed as both endstops)
- `rear-entry.yaml`: winch door as a feedback cover (daisy-chained relays in
  the rocker circuit, two NC endstops, max-runtime timeout)
- `solar-tilt.yaml`: three independent panel actuators as time-based covers
- `enclosure.scad`: parametric printed shell for any node

Parts, pin conventions, and wiring lessons: `docs/hardware.md`. Van
prototype standards (power, connectors, enclosures) and per-node wiring:
`docs/prototype.md`.

## Flashing

```sh
cp secrets.yaml.example secrets.yaml   # fill in wifi + broker + passwords
uvx esphome run electronics-bay.yaml   # first flash over USB, then OTA
```

First boot logs print DS18B20 1-Wire addresses; paste them into the YAML's
`dallas_temp` sensors and reflash. Convention (docs/mqtt-conventions.md):
MQTT mode with no `api:` block, discovery on, hostname `vanifold-<node>`.
