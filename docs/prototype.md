# Van prototype

Moving nodes from the bench into the van before its DC wiring is finished:
battery-powered nodes in printed shells, connected by unpluggable harnesses.
Parts and bench lessons: `docs/hardware.md`.

Build order: rear-entry node, then solar-tilt node. The hub stays rough.

## Standards (every node)

### Power: one 12V interface

Every node takes 12V nominal on the same connector it will use from the van's
12V fuse block. The battery is just a swappable 12V source; moving into the
finished van means changing the source, not the node.

- **Battery**: 12V (12.8V nominal) LiFePO4, 10Ah, BMS rated 15A continuous or
  more. One SKU for every node so packs are interchangeable. LiFePO4 because
  it matches the van's 12V rail voltage, has no auto-shutoff at low draw (USB
  power banks do), and tolerates a hot van far better than other lithium
  chemistries. An idle node draws about 50mA, so roughly 8 days per charge.
- **Battery pigtail**: battery tabs -> inline ATO fuse holder (15A) on the
  positive -> 16AWG -> DT 2-pin plug. The fuse protects the wire, so it lives
  at the battery end.
- **Inside the node**: the MP1584 buck makes 5V for the ESP32's VIN terminal.
  Load circuits (actuators) take the 12V directly.
- **Charger**: one 14.6V LiFePO4 charger.

### Connectors: Deutsch, with two rules

1. **Whoever supplies power gets the sockets.** The battery, the van-side
   winch harness, and the node's own outputs (actuators, sensor pullups) use
   DT06/DTM06 plugs (recessed sockets). Their mates use DT04/DTM04
   receptacles (pins). A disconnected live end never has exposed pins.
2. **Anything carrying battery or load voltage is DT with a pin count unique
   to its job; 3.3V signals are DTM.** A battery can only mate with a power
   port, and a DTM sensor plug cannot physically mate with a DT power port.

| Harness | Connector | Pins |
|---|---|---|
| PWR | DT 2-pin | 1 +12V, 2 GND |
| ACT (one per actuator) | DT 3-pin | 1 lead A, 2 lead B, 3 unused |
| WINCH CTRL | DT 4-pin | 1 feed from control box, 2 feed to rocker, 3 IN line, 4 OUT line |
| SENSE (endstop pair) | DTM 3-pin | 1 GND (both switch COMs), 2 open limit NC, 3 closed limit NC |
| 1-WIRE (temp probes, later) | DTM 3-pin | 1 3V3, 2 GND, 3 data |

SENSE and 1-WIRE share a pin count on purpose: both are 3.3V, so plugging
one into the other is harmless.

Use round jacketed cable (2-4 conductor) so the cable glands seal on it:
16AWG for PWR, ACT and WINCH CTRL; 20-22AWG for DTM signals. Label both ends.

### Enclosure

`nodes/enclosure.scad`: a parametric box with a screw-down lid, PG7 cable
gland holes, floor mounting holes, and optional board standoffs. Export
commands for each node are in the file header (render into `nodes/stl/`,
which is gitignored).

- Each harness leaves the box as a short pigtail (~15cm) through a PG7
  gland, ending in its connector. No connector panel cutouts to get right.
- Boards: VHB tape, or measure their holes and list them in `standoffs`.
- Mount with screws through the two floor holes, or VHB for no-drill testing.

**Prototypes: Formlabs Form 2 (resin).** The shells are prototypes; expect
to reprint as the builds change.

- Both bases are 144.8mm long against a 145mm platform, so they only fit
  tilted 42 degrees or more. That steep tilt is also the right resin
  orientation. Let PreForm auto-orient, then check its cup warnings: if it
  flags the cavity, rotate so the open side faces the build platform (a
  hollow sealed on the platform side and open to the tank creates suction
  on every peel).
- Keep supports off the rim, where the lid seals.
- Resin trade-off: standard Grey resists heat best (~73C post-cured) but is
  brittle; Tough 2000 survives screws and knocks better but softens near
  53C. Either is fine for testing; don't leave a resin shell in a closed van
  through a hot afternoon.
- Lid screws: tap the 2.5mm post holes M3 by hand. Self-tapping screws crack
  resin, and heat-set inserts don't work in it (it doesn't melt). Snug the
  gland nuts; don't crank them.

**Finals: print service.** Once a node's build settles, order its shell in
MJF PA12 nylon (JLCPCB, PCBWay, Xometry and similar): tough, unaffected by
van temperatures, and it takes heat-set inserts. Export with `ears=true` and
`post_hole=4.0`. ASA on an FDM printer also works.

### Network

The Pi and a travel router move as one unit, bench and van alike, so nodes
only ever know one network.

1. Pi on the router by Ethernet; give it a DHCP reservation so the broker IP
   never changes.
2. Put the router's SSID and the Pi's reserved IP in `nodes/secrets.yaml`.
3. Order matters for nodes already flashed: OTA-flash each node with the new
   secrets **while it is still on the old network**, then switch the hub over.
   A node flashed for a network it can't see is only recoverable over USB.

### Hub kit

Rough is fine: Pi 4 and router in any box, powered from the inverter's AC
through their own adapters. Nothing changes on the Pi.

## Rear-entry node

Firmware: `nodes/rear-entry.yaml`. Enclosure: defaults (3 glands: PWR,
WINCH CTRL, SENSE).

### How it connects to the winch

The winch has an integrated control box driven by a momentary rocker. The
node never touches the winch's high-current side. It joins the rocker's
low-current circuit: the rocker's feed wire is cut and routed through two
relays, daisy-chained.

```
feed from control box ── A.COM
                         A.NO ── OUT line   (door lowers onto catch = open)
                         A.NC ── B.COM
                                 B.NO ── IN line    (door rises to jamb = closed)
                                 B.NC ── feed to rocker common

rocker IN/OUT outputs stay connected to the IN/OUT lines as today
relay A = GPIO26 (winch out), relay B = GPIO27 (winch in)
```

What this buys:

- **Node off, dead, or unpowered**: both relays rest on NC, the feed passes
  straight through to the rocker, and the winch works exactly as stock.
- **Hardware interlock**: energizing either relay removes the feed from the
  rocker and from the other relay. IN and OUT can never be energized at the
  same time from any mix of rocker and node (on many winch contactors that
  combination is a dead short). The firmware interlock is a second layer.
- **Bypass dongle**: a DT 4-pin receptacle with pins 1 and 2 jumpered. With
  the node unplugged and the dongle in, the rocker is stock. Tape the dongle
  to the harness; without the node or the dongle, the winch will not move
  (the safe direction to fail).
- **Relay order is deliberate**: relay A (out) is first in the chain, so if
  both relays were ever energized the door lowers onto its catch (slack line)
  instead of straining against the jamb.

Endstops: sealed (IP67) roller-lever limit switches, wired COM to GND and
NC to the pin, so a broken wire reads "at limit" and stops travel. One on the
catch (door resting = open), one on the jamb (door up = closed).

### Identify the rocker wires first

Before cutting anything, with the vehicle's 12V on and a multimeter on DC
volts (black probe on chassis ground):

1. The terminal at a steady 12V with the rocker untouched is the feed.
2. The two that jump to 12V only while the rocker is held one way or the
   other are IN and OUT. Note which direction each one moves the door.
3. Illuminated rockers have extra lamp terminals; leave those alone.
4. If no terminal sits at 12V, the rocker may switch ground instead. The
   relay chain works the same (relay contacts are just switches), but stop
   and confirm the wiring before cutting.
5. Measure the feed current while holding the rocker (meter in series). The
   relays are rated 10A; anything over ~5A needs a different relay.

### Stage 1: monitor only

WINCH CTRL stays in its bypass dongle, so the relays click into nothing.

- [ ] Operate the door with the rocker; "Door on catch" and "Door at jamb"
      follow it on the dashboard.
- [ ] Unplug SENSE: both endstops read "at limit" (fail-safe check).
- [ ] Read the door's travel time each way off the endstop history; set
      `open_duration`, `close_duration`, and `max_duration` (longer travel
      plus ~20%) in the YAML.
- [ ] Note battery runtime.

### Stage 2: control

- [ ] With WINCH CTRL unplugged, reboot the node: both relays stay silent.
      (A wrong `inverted` flag would energize a relay at boot and drive the
      door.)
- [ ] Plug WINCH CTRL in. Node idle: the rocker works. Node unpowered: the
      rocker still works.
- [ ] Dashboard Open: door lowers and stops on the catch switch. Close: door
      rises and stops at the jamb switch.
- [ ] Stop mid-travel from the dashboard: halts immediately.
- [ ] Timeout: temporarily set `max_duration: 3s`, command a move, confirm it
      stops at 3s, then restore the real value.
- [ ] Mark the cover `safety` (PATCH command in the YAML header).

Known limits, by design for now:

- **While the node drives the door, the rocker is dead** (that is the
  interlock). Stop paths: dashboard Stop, the endstops, `max_duration`, or
  unplugging the node's PWR (relays drop to NC, the winch stops, the rocker
  comes back).
- **Manual moves show only at the ends**: the cover updates when an endstop
  trips, not while the rocker is moving the door.
- **No stall detection**: if the jamb switch fails, the winch pulls against
  the jamb until `max_duration`. Keep that value tight. The upgrade is a
  split-core DC hall sensor clamped on the winch cable (the ACS758 would need
  the cable cut and routed through it).

## Solar-tilt node

Firmware: `nodes/solar-tilt.yaml`. Enclosure: `inner=[180,120,45]`, 4 glands
(PWR + 3 ACT); export command in the scad header.

### Wiring

Each actuator gets the relay reversal pair proven on the bench: lead A on
one relay's COM, lead B on the other's; every NO goes to +12V, every NC to
GND. Idle = both leads on GND (braked). A wrong relay state can only put both
leads on the same rail, which does not move the actuator.

- Relay pairs: GPIO26/27 (tilt 1), 18/19 (tilt 2), 22/23 (tilt 3).
- Distribute +12V and GND inside the box with Wago 221 lever nuts, 16AWG.
- If "up" moves an actuator down, swap its leads at the ACT plug.

**Relay board: one 8-channel opto board with 5V coils and a JD-VCC jumper.**
Three actuators can run at once, which means three coils energized. Fed from
the ESP32's small onboard 3.3V regulator (as the 1ch modules are), that is
too much on top of the ESP32 itself. Remove the JD-VCC jumper, feed JD-VCC
from the buck's 5V, feed VCC from 3V3, inputs from the GPIOs. These boards
are usually active-low: if relays click on at boot, flip every `inverted`.

**Current budget**: read each actuator's rated current off its label. Three
at once must stay under the battery BMS rating (15A pack covers three ~4A
actuators). Actuator current flows through the PWR harness, which is why PWR
is 16AWG with a 15A fuse.

### Bring-up

- [ ] Bench first, actuators on the bench: each tilt moves both ways, the
      built-in limit stops it, the relay releases after the timer.
- [ ] Time each full stroke under load; set durations to that plus ~10%.
- [ ] Reboot with actuators connected: nothing moves.
- [ ] Kill WiFi mid-travel (unplug the router): the run finishes on its own.

Not built yet, but the next safety feature here: **ignition-sense auto-stow**.
Driving with a panel tilted up risks tearing it off. An opto-isolated input
from the vehicle's ignition line (GPIO13 reserved) lets the node close all
three panels and refuse "open" while the ignition is on. That is Tier 2 logic
and belongs on this node, not the hub.

## Shopping list

| Item | Qty | For |
|---|---|---|
| 12V 10Ah LiFePO4, BMS 15A+ continuous | 2 | One per node |
| 14.6V LiFePO4 charger | 1 | |
| Inline ATO fuse holder, 16AWG, + 15A fuses | 2 | Battery pigtails |
| Deutsch DT/DTM kit with matching crimper (2, 3, 4-pin DT; 3-pin DTM) | 1 | All harnesses. Check the crimper matches the kit's contact type (stamped vs solid) |
| PG7 cable glands | 10 | Enclosures |
| IP67 roller-lever limit switch | 2 | Catch and jamb endstops |
| 8ch 5V opto relay board with JD-VCC jumper | 1 | Solar tilt |
| MP1584 buck | 1 more | One per node |
| Round jacketed cable: 2-4 conductor 16AWG and 20-22AWG | some | Harnesses |
| Wago 221 lever nuts | 1 pack | Power distribution in the solar box |
| M3 x 8mm screws, M3 hand tap | | Enclosure lids (resin prototypes) |
| Travel router (GL.iNet class) | 1 | Van LAN, if not already owned |
