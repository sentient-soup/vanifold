# nodes/

ESPHome configurations for vanifold nodes: distributed processors placed near each
subsystem cluster. Per the three-tier law (docs/DESIGN.md), control loops and
failsafes run ON the node; the hub only orchestrates.

Planned v0.1 nodes:
- `electronics-bay`: DS18B20 battery temps, relay, PWM light strip channel
- `rear-entry`: winch door as a feedback cover (interlocked solenoid relays,
  limit switches, current-sense stall cutoff, max-runtime timeout)
