# MQTT Conventions (draft v0.1)

How everything meets on the spine. The core daemon speaks only MQTT; if a device
cannot reach the broker, an adapter (node firmware or sidecar bridge) gets it there.

Status: DRAFT. Open questions at the bottom.

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
3. Core publishes the entity's declared `command_topic` payload, QoS 1.
4. State is **confirmed, not optimistic**: the entity updates when the device
   echoes its state topic. The UI shows a pending indicator until echo or timeout
   (default 5s), then surfaces failure. Devices flagged `optimistic` in their
   discovery payload are the only exception.

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

## Open questions (tear these apart)

1. Command echo timeout: 5s default, but winch travel takes ~30s+. Covers report
   `opening`/`closing` intermediate states, which solves it for kind `cover`;
   is a per-kind timeout table enough?
2. Should the core republish a normalized state tree under `vanifold/state/#`
   for third-party consumers, or is the WebSocket API the only egress in v0.1?
   Draft: WebSocket only; republish when the HA-export bridge lands.
3. Retained command topics are a known foot-gun (replayed actuations after
   reconnect). Draft: core never retains commands and refuses to actuate from a
   retained command echo. Any counter-case?
