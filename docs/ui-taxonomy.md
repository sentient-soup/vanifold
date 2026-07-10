# UI Taxonomy and Interaction Hierarchy

Status: ACCEPTED 2026-07-10. This document reworks the original subsystem list
around how van/RV living actually flows, and defines the interaction hierarchy
the UI is built from. The baseline is a normative picture of van life, not any
one owner's habits; owner-specific tuning comes later via entity pinning and
threshold config.

## The domain, observed

Van life runs on a small number of recurring mental loops:

1. **Resource anxiety.** Energy and water are the two consumables that gate
   everything else. Owners check state of charge the way phone users check
   battery: many times a day, wanting one number and a trend, not a schematic.
   Water is the same loop at lower frequency: fresh remaining, grey capacity
   remaining, both of which drive route planning (fill and dump stops).
2. **Comfort setpoints.** Temperature is the metric people care about
   remotely (pets asleep in the van while shopping) and the control they touch
   daily (heater, fan, AC). It is a *setpoint* interaction: current value,
   target, adjust.
3. **Openings.** Doors, locks, windows, and powered movers (rear entry, bed).
   The nightly question is binary: is everything closed and locked? The
   controls are momentary and consequential, some safety-tier.
4. **Light.** The most frequent control interaction of all, but also the one
   with the best physical fallback (switches stay wired, per the three-tier
   law). The panel's job is scenes and zones ("everything off", "dim for
   night"), not replacing the switch by the door.
5. **The vehicle itself.** Fuel, starter battery, tires: a *pre-drive*
   checklist loop, plus drive-mode consequences for the rest of the system
   (interlocks). Checked before moving, ignored while parked.
6. **Situational subsystems.** Media/AV, cameras, diagnostics: real but
   occasional; they must not occupy glance-layer real estate.

Two cross-cutting observations shape the design language:

- **Levels vs switches vs states.** Everything surfaced at the top is one of:
  a *level* (SoC, tanks, fuel: render as gauge, communicate rate and
  time-until), a *switch/setpoint* (pump, thermostat, lights: render as a
  large touch control), or a *security state* (locked/open: render as a
  binary summary that can say "all secure" in one glyph).
- **The van has modes.** Parked, driving, away, night. Modes gate interlocks
  (spec'd already via vehicle state) and eventually re-prioritize the UI.
  v0.1 renders vehicle state where known and nothing more.

## Taxonomy v2

| Key | Panel | Contains | Frequency |
|---|---|---|---|
| `power` | Power | battery SoC/voltage/temps, solar input, inverter, circuits, bay fan | Glance: many/day. Control: rare |
| `water` | Water | tank levels, pump, water temps, valves | Glance: daily. Control: pump daily, valves weekly |
| `climate` | Climate | inside/outside temp, humidity, heater, vent fans, AC | Glance+control: daily |
| `lighting` | Lighting | all light entities, future scenes | Control: many/day |
| `access` | Access | doors, locks, windows, rear entry, bed, awning | Glance: nightly. Control: daily, some safety-tier |
| `vehicle` | Vehicle | fuel, starter battery, ignition/drive state, tires | Pre-drive |
| `media` | Media | computer power, audio, KVM, cameras-as-viewers | Situational |
| `misc` | (catch-all) | anything unassigned | n/a |

Changes from v1: `plumbing` renamed `water` (owner vocabulary, not trade
vocabulary); `computer` renamed `media` (what it is to the user); `lighting`
and `access` promoted to first-class subsystems (they were buried in misc,
yet lighting is the most-touched control and access is the nightly ritual);
`electronics` was already `power`.

## Interaction hierarchy

**Layer 0 - status strip (always visible):** clock, connection lamp, alert
badges, vehicle state when known. Alerts are the only thing allowed to
interrupt: battery low, grey nearly full, fresh nearly empty, safety entity
offline.

**Layer 1 - home (the glance).** One screen, no scrolling on a landscape
kiosk. Energy is the hero (biggest tile: SoC arc, net watts). Water and
climate are second tier. Access summary, lighting shortcuts, vehicle fuel are
third tier. Every tile is a door into its panel; nothing on the home screen
is informational-only decoration.

**Layer 2 - subsystem panels.** Each panel leads with its hero reading
(gauge-scale, glanceable from across the van), then its daily-driver controls
as large touch targets, then the full entity inventory grouped by device.

**Layer 3 - entity detail.** The existing drawer: history chart, metadata,
quality, admin. The escape hatch to full granularity.

**System/diagnostics** (device list, quarantine) live behind a settings entry
in the status strip, not in the subsystem taxonomy.

## Surfacing rules (what gets a big control)

1. Levels with thresholds always beat raw sensors for tile space.
2. A subsystem's daily-driver control rides on its panel hero: pump on the
   Water panel, thermostat on Climate, all-off on Lighting.
3. Safety-tier controls (rear entry, bed, valves) are large but deliberate:
   distinct styling, never adjacent to high-frequency toggles, room for
   future confirm/hold semantics.
4. Sensors that back a gauge (the SoC%, the tank %) do not repeat as rows in
   the panel inventory unless expanded.

## Featured-metric heuristics (v0.1)

Tiles pick their hero metrics by priority rules over device_class/kind, so
any van's devices produce a sane dashboard with zero config:

- Power: SoC = first `battery` %; voltage = `voltage` on the same device;
  flow = sum of `power` entities (labeled "measured", direction-agnostic
  until power topology config exists).
- Water: levels = entities in `water` with % unit (fresh vs grey by name
  hint); pump = first switch in `water`.
- Climate: inside temp = first `temperature` in `climate`; humidity ditto.
- Access: "all secure" = no door/window/lock binary in a notionally-open
  state; movers listed individually.
- Vehicle: fuel = first `%` in `vehicle`.

Name hints ("fresh", "grey/gray", "solar", "battery") are evaluated after
unambiguous device classes but before ambient ones, so "Battery temp" lands in
power rather than climate. All suggestions are user-overridable via entity
metadata. When a hero metric is missing the tile degrades honestly (shows
what it has, never invents).

## Default alert thresholds (opinionated, config later)

- Battery SoC: warn < 20%, critical < 10%
- Fresh water: warn < 15%
- Grey water: warn > 85%
- Any `criticality: safety` entity unavailable: warn
- Battery temp: warn > 45°C

## Subsystem auto-suggestion (core)

`suggest_subsystem` maps to taxonomy v2: battery/current/voltage/power/energy
-> power; temperature/humidity -> climate; door/window/lock/garage_door ->
access; kind light -> lighting; moisture/water -> water; name hints
(water/tank/fresh/grey) -> water, (solar/battery/inverter) -> power, (fuel) ->
vehicle. Suggestions only; user assignment always wins and survives
re-announcement.
