# MQTT Conventions (v0.1)

How everything meets on the spine. The core daemon speaks only MQTT; if a device
cannot reach the broker, an adapter (node firmware or sidecar bridge) gets it there.

Status: ACCEPTED. Former open questions resolved 2026-07-09; resolutions inlined
in their sections.

## Broker

- Mosquitto on the hub, port 1883, van-LAN only. Not internet-exposed, ever;
  remote access is a VPN problem (Tailscale-class), not a broker problem.
- v0.1 auth: single username/password shared by devices, distinct credentials for
  the core. Per-device credentials and TLS are documented as deferred, not
  designed away.

## Namespaces

| Topic space | Who publishes | Purpose |
|---|---|---|
| `homeassistant/<component>/[<node_id>/]<object_id>/config` | devices, bridges | HA MQTT Discovery announcements (retained). The ingestion surface. |
| device-declared topics | devices | state/command/availability topics named inside each discovery payload. We follow the payload; we impose nothing. |
| `vanifold/core/status` | core | `online`/`offline` (LWT, retained). |
| `vanifold/bridge/<name>/status` | bridges | bridge health, same contract as core status. |

The `homeassistant/` prefix literal is kept even though HA is not present: every
existing firmware lets you set the discovery prefix, and defaulting to the value
the whole ecosystem already uses IS the plug-n-play feature.

## Discovery ingestion rules

1. Subscribe `homeassistant/#`. A retained `config` payload creates or updates a
   Device + Entity; an empty retained payload deletes (per HA convention).
2. **Abbreviation expansion first**: HA payloads may use short keys (`stat_t`,
   `cmd_t`, `avty_t`, `dev`, `uniq_id`, ...). One table in the discovery module
   expands them; nothing downstream sees abbreviations.
3. **`unique_id` is required.** Payloads without it go to quarantine (stored,
   shown in diagnostics with a reason, not registered). Same for unparseable
   payloads and unknown components. Nothing is silently dropped.
4. Device grouping via the `device` block (identifiers/connections), exactly as
   HA does it, so multi-entity devices (a Shelly 3EM's three channels) coalesce.
5. Availability: honor `availability_topic` / LWT payloads; absence of one means
   quality degradation only via `stale_after` heuristics.
6. Value extraction: v0.1 supports plain payloads and `value_template` limited to
   the overwhelmingly common form (`{{ value_json.<path> }}`). Full Jinja is
   explicitly out of scope; templates we cannot evaluate quarantine the entity
   with a clear reason. (Expect this to cover nearly everything ESPHome, Shelly,
   Tasmota, and zigbee2mqtt emit.)

## Command flow

1. UI/API issues a command against an entity.
2. Core validates against tier-3 interlocks (reject with a reason, never queue
   silently).
3. Core publishes the entity's declared `command_topic` payload, QoS 1, retain
   never set, with an MQTT v5 message expiry interval of 10s (see below).
4. State is **confirmed, not optimistic**: the entity updates when the device
   echoes its state topic. The UI shows a pending indicator until echo or timeout
   (uniform 5s across all kinds), then surfaces failure. Devices flagged
   `optimistic` in their discovery payload are the only exception.

Acknowledgment vs completion: the 5s window is only "did the device hear me".
Long-running motion (a 30s winch travel) is a state concern, not a command
concern: the cover sits in `opening`/`closing` and its travel timeout is the
node's Tier 2 responsibility, never the hub's. The hub flags an entity stuck in
a transitional state for over 5 minutes as a diagnostics warning (misbehaving
node), not an enforcement action.

## Command replay safety

Two broker mechanisms can actuate a device at a moment nobody asked for:

- **Retained commands**: a retained `open` on a command topic is re-delivered
  every time the subscriber reconnects (the 3am-winch scenario). The core never
  sets retain on command publishes and never treats a retained message as a live
  actuation trigger. Absolute in v0.1. A per-entity `retain_commands` opt-in for
  memoryless third-party gear is a documented future escape hatch, refused for
  `criticality: safety` entities, and not built until a real device demands it.
  Devices we build restore desired state locally (ESPHome `restore_mode`), per
  the Tier 2 principle: state persistence belongs on the node, not the broker.
- **Offline-queue replay**: QoS 1 commands published while a persistent-session
  subscriber is offline are queued and delivered on reconnect, minutes or hours
  later. The retain ban does not cover this path; the MQTT v5 **message expiry
  interval** (10s) on every command publish does. The broker discards expired
  commands from queues and retained storage regardless of subscriber firmware or
  protocol version, so this protects third-party devices too.

## QoS and retention

- Discovery configs: retained, QoS 1 (publishers' norm; we require retained).
- States: we accept whatever devices do. Retained states ingest with
  `quality: retained` (age unknown) until the first live update.
- Core/bridge status topics: retained, QoS 1, with LWT for `offline`.

## Node and bridge conventions (things we build)

- ESPHome nodes: MQTT mode (no native API block), discovery on, default prefix,
  birth/LWT configured, hostname pattern `vanifold-<node>` (e.g.
  `vanifold-rear-entry`). Tier-2 logic lives in the node YAML; a node must behave
  sanely with the broker unreachable (failsafe output states declared per node).
- Bridges: publish standard HA discovery for their devices plus
  `vanifold/bridge/<name>/status`. A bridge is indistinguishable from a native
  MQTT device by design; the core has no bridge-specific code paths.

## Resolved questions (2026-07-09)

1. **Command timeouts**: no per-kind table. One uniform 5s acknowledgment
   timeout; travel/completion time is node-owned (Tier 2). Inlined above.
2. **Normalized state republish**: rejected for v0.1. WebSocket/REST is the only
   egress; a public `vanifold/state/#` tree would be a second forever-stable API
   frozen before the internal model has met real devices, and the raw device
   topics are already on the broker for tinkerers. The HA-export bridge becomes
   the normalized MQTT egress when it lands, built once for a concrete consumer.
3. **Retained commands**: ban is absolute, plus MQTT v5 message expiry (10s) on
   all command publishes to also close the offline-queue replay path. Inlined in
   "Command replay safety". (Deadline-as-user-properties schemes from industrial
   SDN-MQTT research were considered and rejected: user properties are opaque to
   the broker and require subscriber cooperation, which third-party devices will
   never provide; broker-enforced expiry gives the useful half for free.)
