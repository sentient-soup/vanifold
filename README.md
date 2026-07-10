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

## Install

On a Raspberry Pi (or any Debian-family box) - installs the broker, the hub
binary for your architecture from the latest release, a systemd service, and
runs an end-to-end smoke test:

```bash
curl -fsSL https://raw.githubusercontent.com/sentient-soup/vanifold/main/scripts/install.sh | sudo bash
```

Re-run the same command to update. Pin a version with
`VANIFOLD_VERSION=v0.1.0`, or build from source with
`sudo ./scripts/install.sh --source` (needs Rust + Node 20+).

## Develop

- `scripts/build.sh` - one-stop local build: UI bundle + single release
  binary with the UI embedded.
- `npm run dev --prefix ui` - UI dev server with hot reload; point it at a
  running hub with `VANIFOLD_API=http://vanhub.local:8480`.
- Open the UI with `?demo` for a simulated van, no hardware needed.
- Releases are tag-driven: bump `core/Cargo.toml`, tag `vX.Y.Z`, push the
  tag; CI cross-compiles aarch64/armv7/x86_64 artifacts and publishes them.

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
