# vanifold

One hub to unify a van/RV's sprawling subsystem controls. Instead of 20 knobs,
switches, and vendor apps, one box: a Rust daemon on any small Linux machine,
distributed ESP32 nodes near each subsystem, and a dashboard that actually sells
the idea.

**Status: v0.1 foundation.** Empty building, finished blueprints. See
[docs/DESIGN.md](docs/DESIGN.md) for the founding decision record.

## How it works

- Everything meets on an **MQTT spine**. The core daemon speaks only MQTT.
- Plug-n-play via the de facto standard: **Home Assistant MQTT Discovery**
  ingestion. Anything that "works with HA over MQTT" (ESPHome, Shelly, Tasmota,
  zigbee2mqtt, zwave-js-ui, WLED) appears with zero code. No Home Assistant
  required, or included.
- **Three-tier autonomy law**: the hub is never in the critical path. Hardware
  guards (Tier 1) and node-local control loops (Tier 2) keep the van safe and
  livable with the hub dead; the hub (Tier 3) only orchestrates.

## Layout

| Path | What |
|---|---|
| `core/` | Rust daemon: MQTT ingestion, entity registry, WebSocket/REST API, SQLite history |
| `ui/` | SvelteKit dashboard, static build embedded into the core binary |
| `bridges/` | Sidecar adapters for non-MQTT devices (BLE BMS, Victron BLE, ...) |
| `nodes/` | ESPHome configs for the distributed nodes |
| `docs/` | Specs: [design record](docs/DESIGN.md), [entity model](docs/entity-model.md), [MQTT conventions](docs/mqtt-conventions.md) |

## License

Code is dual-licensed under MIT or Apache-2.0, at your option.
