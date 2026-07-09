# Vanifold - Design Record

One hub to unify a van/RV's sprawling subsystem controls. Open-source design others
can replicate; the author's van is the v0.1 testbed. Full feature surface is spec'd
up front at the interface level; features light up as they're installed in the van.

## Locked decisions

1. **Topology: hub + distributed nodes.** Nodes are distributed processors
   (ESP32/ESPHome), not dumb I/O. New subsystem = drop a node near it.
2. **Standalone backend, not Home Assistant.** Research (July 2026) confirmed ~90%
   of HA's van-relevant device value is reusable outside HA (standalone PyPI libs,
   ESPHome components, zigbee2mqtt/zwave-js-ui, Venus OS native MQTT). The rebuild
   cost is only the platform layer (registry, history, automations), which is where
   van-specific value lives anyway.
3. **MQTT spine.** The core daemon speaks only MQTT. Every protocol lives in an edge
   adapter: ESPHome nodes, zigbee2mqtt, zwave-js-ui, Venus OS, and sidecar bridge
   daemons wrapping existing libs (victron-ble, aiobmsble, mopeka-iot-ble, ...).
4. **Core language: Rust.** Single static binary, embedded UI assets, low idle power.
   Target: any aarch64/x86_64 Linux. Reference hub: Raspberry Pi 4.
5. **Plug-n-play = HA MQTT Discovery ingestion.** Anything that "works with Home
   Assistant over MQTT" (ESPHome, Tasmota, Shelly, WLED, z2m, zwave-js-ui) plugs in
   with zero code. All payload ugliness is quarantined in one discovery module.
   Internal entity model is a clean superset of HA entity semantics plus van-native
   concepts (power budgets, tank topology, drive-mode interlocks).
6. **THE LAW - three-tier autonomy. The hub is never in the critical path.**
   - Tier 1: physics/hardware - fuses, BMS cutoffs, limit switches, spring-return.
   - Tier 2: node-local control loops - thermostat runs ON the node, comms-loss
     failsafe states, procedures (winch travel) execute locally.
   - Tier 3: hub orchestration - scenes, budgets, history, UI, remote access.
   The core REFUSES to configure a safety interlock that exists only at Tier 3.
   Anything that can injure (bed elevator, winch door) requires a Tier 1/2 guard
   before software drives it. Physical switches stay wired, rerouted through nodes.
7. **Network:** dedicated router owns the van network; hub is a client (Ethernet
   preferred). WiFi is for powered ESPHome nodes doing real work; coin-cell
   Zigbee (later Z-Wave if a device earns it) for scatter sensors.
8. **Frontend: SvelteKit** (Svelte 5 runes, adapter-static, @vite-pwa/sveltekit),
   embedded in the core binary. WebSocket-first live data; REST for config/history.
   Dark mode from day one. The UI is a flagship feature, not an afterthought.
   (Rust/WASM frontends evaluated and rejected July 2026: Leptos in light-
   maintenance mode, Dioxus API churn, 5-20x larger payloads, thin chart/gauge
   ecosystem; WASM energy advantage doesn't apply to DOM-bound workloads.)
9. **Persistence: SQLite** (WAL, rusqlite), one file for config + registry +
   history. Tiered downsampling (raw ~48h -> 1-min ~30d -> 1-hour forever).
   Batched writes; SD-wear-conscious. External TSDBs are optional exports, never
   dependencies.

## Repo layout (monorepo)

- `core/` - Rust workspace: MQTT ingestion, entity model, API, persistence
- `ui/` - SvelteKit app, static build embedded into core binary
- `bridges/` - sidecar adapters (Python BLE bridges etc.)
- `nodes/` - ESPHome YAML configs, eventually custom node hardware designs
- `docs/` - specs: entity model, MQTT conventions, tier law, subsystem map

## v0.1 scope

Functional:
1. Core pipeline end-to-end: MQTT -> HA-discovery ingestion -> entity registry ->
   WebSocket API -> SQLite tiered history.
2. UI: live auto-generated dashboard from discovered entities, switch/light/cover
   control, dark mode.
3. Shelly 3EM on the AC load panel - zero-code discovery validation.
4. ESP32 node #1: DS18B20 battery temps + relay + PWM light strip channel.
5. ESP32 node #2 (rear entry): winch door as ESPHome feedback cover - relays on
   solenoid coils (firmware-interlocked), limit switches, current-sense stall
   cutoff, max-runtime timeout, existing manual switch rewired through the node.
6. Experiment: BLE scan of Portable Sun battery with aiobmsble (suspected
   white-label JBD/Daly BMS) -> becomes bridge #1 if it talks.

Scaffolded only (spec + API surface, not functional): automation/interlock engine
(with Tier-3-refusal designed in), Zigbee/Z-Wave (works by construction via
z2m/zwave-js-ui; untested), energy budgeting, vehicle/CAN (WiCAN later), cameras
(separate box, Frigate-class), HA-export bridge.

Not started: custom wide-input carrier PCB (48V van: note "12V-native" concept is
now "wide-input"), reference touchscreen device, bed elevator (blocked on Tier 1
mechanical safeguard).

## Owner's van context

48V system: 6x panels series -> SPH6548P all-in-one inverter (RS485/Modbus port =
future custom adapter); 2x 48V batteries -> bus bars -> 160A breaker -> inverter -> AC
panel; 48->12V DC-DC (Victron Orion planned) -> 12V fuse block; winch direct off
vehicle 12V. DC distribution still being built - adjustable.

Hardware on hand: RPi 4 (hub) + RPi 3 (bench), Shelly 3EM + 3x 120A CTs, 3x DS18B20
probes, Amcrest IP cam (parked), Arduinos (bench only, no radios).
Shopping list: 2-3 ESP32 devkits, limit switches, Zigbee dongle (when needed).
