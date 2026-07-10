# Entity Model (draft v0.1)

The internal data model of vanifold-core. A clean superset of Home Assistant's
entity semantics: everything an HA MQTT Discovery payload can describe maps into
this model losslessly, and van-native concepts extend it. Nothing outside the
discovery module ever sees an HA payload.

Status: ACCEPTED 2026-07-09, with the draft answers to the questions at the
bottom confirmed as decisions.

## Object hierarchy

```
Subsystem (power, plumbing, climate, vehicle, computer, misc)
  └─ Device (one physical thing: a node, a Shelly, a BMS)
       └─ Entity (one typed state/command unit: a sensor, a switch, a cover)
```

- **Device**: identity (stable `device_id`), manufacturer/model/firmware metadata,
  connectivity source (which adapter announced it), availability. Devices arrive
  via discovery; the user never hand-registers one to get telemetry.
- **Entity**: the atom. Belongs to exactly one device. Assigned to exactly one
  subsystem (auto-suggested from device_class/unit, user-overridable).
- **Subsystem**: van-level grouping used by the UI and by cross-subsystem logic
  (energy budgets, drive-mode rules). Fixed taxonomy v2 (see ui-taxonomy.md):
  `power, water, climate, lighting, access, vehicle, media, misc`. Not
  user-extensible until a real need appears.

## Entity anatomy

| Field | Notes |
|---|---|
| `entity_id` | Stable, derived from the source's `unique_id` (required; see MQTT conventions). Never derived from display names. |
| `device_id` | Owning device. |
| `kind` | See kind table below. |
| `name` | Display name, user-editable, never used as identity. |
| `state` | Typed value + `updated_at` timestamp + `quality`. |
| `quality` | `live` (fresh push), `retained` (last known from broker retention, age unknown), `stale` (no update within `stale_after`), `unavailable` (device offline via LWT). The UI must render quality honestly; a stale tank level looks different from a live one. |
| `unit`, `device_class` | Adopted from HA vocabulary verbatim (battery, temperature, power, ...). No parallel vocabulary invented. |
| `commands` | The writable surface, per kind (e.g. switch: on/off; cover: open/close/stop/position; light: on/off/brightness). |
| `criticality` | `info` (default), `comfort`, `safety`. Safety-critical entities get UI prominence and are the subjects of interlock rules. |
| `attributes` | Open key-value bag for source-specific extras. Escape hatch, not a design surface. |

## Entity kinds (v0.1 set)

Adopted directly from HA component semantics. v0.1 ingests:

| Kind | State | Commands |
|---|---|---|
| `sensor` | numeric or text + unit | none |
| `binary_sensor` | on/off + device_class | none |
| `switch` | on/off | on, off |
| `light` | on/off, brightness (color deferred) | on, off, brightness |
| `cover` | open/closed/opening/closing (+ position) | open, close, stop (+ set_position) |

Recognized but deferred (parse, store, no UI/commands yet): `climate`, `lock`,
`number`, `select`, `button`, `fan`, `camera`, `event`. Unknown components are
retained raw in quarantine, visible in a diagnostics view, never dropped silently.

## Van-native extensions

These are vanifold's reason to exist. They are overlays that reference entities;
they do not replace them.

### Tank
A composite object binding related entities into one concept the UI can render as
one thing: `level` (sensor), `capacity` (config), optional `temperature`,
`flow_rate`, and associated valve entities (covers or switches). Fresh/grey/black
typed. Tank topology (which valve drains which tank into what) is config, not
discovery.

### Power topology
Every power-ish entity can be tagged with a role: `source` (solar, alternator,
shore), `storage` (battery), `bus` (48V bus, 12V bus, AC panel), `load`.
This is the substrate for energy accounting and, later, budget enforcement
("shed loads when storage below X"). Topology is config; roles are auto-suggested
from device_class where possible.

### Interlock
A named rule constraining commands, with a declared **tier** (see DESIGN.md law):

- `tier: 1` documents a hardware guard (limit switch, fuse). Informational; the
  core cannot verify it but records that it exists.
- `tier: 2` documents a node-local guard (ESPHome-side logic). Also recorded,
  optionally verified by node config linting later.
- `tier: 3` is a hub-enforced command filter (e.g. "reject grey-valve open unless
  vehicle state is parked").

**The law, enforced here**: an interlock whose subject entity has
`criticality: safety` MUST reference at least one tier-1 or tier-2 guard.
Configuring a safety interlock that exists only at tier 3 is a rejected config,
not a warning.

### Vehicle state
A first-class system entity: `parked | ignition_on | driving | unknown`, fed by
whatever source exists (ignition-sense GPIO on a node in v0.1; CAN later).
`unknown` is a legal state and interlocks must declare their behavior under it
(default: fail closed, i.e. treat unknown as driving).

## Persistence mapping

Entities and devices live in SQLite (registry tables). State history goes to the
tiered history tables (raw ~48h, 1-min ~30d, 1-hour indefinitely) keyed by
`entity_id`. Only numeric and on/off states are historized by default; text and
attribute churn are not.

## Resolved questions (2026-07-09, draft answers confirmed)

1. Subsystem assignment lives on the **Entity**, because one node hosts entities
   from multiple subsystems.
2. `criticality` lives on the **entity**, so the UI can treat safety things
   specially even before any interlock exists.
3. **Tank is hardcoded** as the first composite; generalize into user-defined
   "fixtures" only when a second composite actually appears.
4. Zones/locations (cab, galley, garage) are **deferred**; subsystems carry v0.1.
