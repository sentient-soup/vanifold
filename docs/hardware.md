# Hardware

What we use, why, and what the bench taught us. Status reflects the October
2026 bench session (Pi 4 hub on v0.2.1, two ESP32 configs in `nodes/`).

## Parts register

| Part | Role | Status | Notes |
|---|---|---|---|
| Raspberry Pi 4 | Hub (Mosquitto + vanifold-core) | Proven | Installed via `scripts/install.sh`; dashboard :8480, broker :1883 |
| Raspberry Pi 3 | Spare / second bench hub | Unused | |
| Shelly 3EM (Gen1) + 3x 120A CT | AC circuit metering | Proven | Gen1 publishes no HA discovery; `scripts/shelly-3em-discovery.sh` announces it once (retained) |
| ESP32 devkit (esp32dev) x3 | Node MCU | Proven | ESPHome, MQTT mode, esp-idf framework |
| Screw-terminal ESP32 breakout | Node carrier | Proven | Strip and tin stranded wire; loose strands caused every flaky contact we saw |
| DS18B20 waterproof probe (Gikfun) x3 | Battery / bay temps | Proven | Red VDD, black GND, yellow data. One 4.7k pullup per bus, not per probe |
| 1ch opto relay module (3V coil) | Switched loads, motor reversal | Proven | Active-high (`inverted: false`). Coil VCC from 3V3; contacts are a dry switch, any load voltage within rating |
| D4184 MOSFET module ("jzmos") | PWM dimming of 12V analog strips | Wired, unloaded | VIN/VOUT screw terminals; 4 small holes are 2 GND + 2 TRIG/PWM (paired for daisy-chaining); leave J1 empty (it bypasses to always-on) |
| MP1584EN buck ("4R7" inductor) | 5V rail | Proven | 5-30V in, fixed 5V / 1.8A out |
| JGA25-370 6V 133rpm gearmotor | Winch stand-in | Proven | Runs fine at 5V (~110rpm) |
| Reed switch, microswitch (limit) | Contact / endstop sensing | Proven | Switch to GND, internal pullup, 20ms debounce |
| WS2812 3-pixel strip (5V) | Addressable test article | Proven | Bench only; see LED decision below |
| ACS758 100A hall sensor | Winch stall detection | Shelved | Current must flow through the IC, so a winch cable would have to be cut and routed through it; a split-core DC hall sensor clamps on instead |
| LM2596 buck | Spare adjustable rail | Unused | |
| Amcrest IP camera | Future camera phase | Parked | |

## ESP32 pin conventions

Same pin for the same job on every node, so harnesses and YAML stay
interchangeable.

| GPIO | Job | Why this pin |
|---|---|---|
| 4 | 1-Wire bus (DS18B20) | General purpose, no boot role |
| 25 | PWM light channel (D4184) | LEDC-capable, no boot role |
| 26, 27 | Relay pair 1 (interlocked when reversing) | No boot role, safe idle state |
| 18, 19 | Relay pair 2 | No boot role |
| 22, 23 | Relay pair 3 | No boot role |
| 32, 33 | Contact inputs (endstops, reed) | Internal pullups available |
| 13 | Ignition sense (opto input), reserved | No boot role, internal pullup |
| 34 | Analog current sense, reserved | Input-only ADC pin, fine for analog |
| 15, 16 | Addressable strip data | 15 is a strapping pin (only affects boot log), prefer 16 |

Avoid for switches: GPIO34-39 (input-only, no internal pullups, a switch floats).
Avoid for outputs: strapping pins 0, 2, 5, 12, 15 (they can glitch at boot).

## Decisions and reasoning

**Shelly Gen1 discovery by script, not core code.** The core consumes HA
discovery and nothing else; teaching it Shelly topics would put a vendor
special case in the spine. A one-shot retained announcement keeps the 3EM
zero-code. Gen2+ Shellies publish discovery natively.

**Analog LED strips for lighting, addressable for accents only.**
Addressable pixels draw about 1mA each while showing black (a 300-pixel run
idles near 300mA, a parasitic load an off-grid battery notices), their white
is low-CRI RGB blending, one dead pixel can black out everything downstream,
and the single-ended data line picks up van noise (inverter, winch, pump). A
PWM-dimmed analog strip draws nothing at 0%, offers high-CRI warm or tunable
white, and degrades gracefully. If an accent zone goes addressable, use 12V
WS2815 (backup data line, tolerates length), never 5V for long runs.

**Two SPDT relays reverse a DC motor.** Each motor lead sits on a relay COM
that rests on GND (NC) and switches to supply (NO). One relay alone drives a
direction; both idle shorts the motor to ground, which brakes it. This is the
same topology as the real winch's two solenoid coils, so the bench exercises
the real failure modes. Firmware interlock with a 300ms break-before-make gap
means both directions can never energize together.

**Endstops on NC contacts in production.** Wired COM to GND, NC to the pin, a
broken wire reads the same as "limit reached", so a damaged harness stops
travel instead of running past the end. (Bench sensors used NO for intuitive
press-equals-on readings.)

**Actuator relays are `internal: true`.** The cover entity is the only thing
the hub can command; nobody can bypass its state machine by toggling a raw
relay from the dashboard.

**Relay coil and contacts are separate circuits.** Coil side (VCC/GND/IN)
must match the coil rating printed on the relay cube (`SRD-03VDC` = 3V,
`SRD-05VDC` = 5V). Contact side (COM/NO/NC) is a mechanical switch and never
touches the coil. With the coil on the ESP32's 3V3, the load supply needs no
shared ground.

**Shared ground is required when a signal crosses supplies.** The D4184's
TRIG/PWM input is referenced to its own GND, so the 12V supply ground and the
ESP32 ground must be tied, or the gate floats.

## Bench lessons

- **Entities appearing proves discovery, not data.** "No numeric history"
  meant the DS18B20 sensors still had placeholder addresses. Check
  `esphome logs` for `Unable to select an address` first.
- **Scratchpad CRC errors with all-FF bytes** mean the configured address
  is not on the bus (wrong or disconnected probe), not a wiring fault.
- **1-Wire is a parallel voltage bus.** Position in the chain does not matter
  at bench lengths; a probe that won't enumerate is a contact problem.
- **ESPHome lights use the JSON schema by default.** Real hardware found a
  core gap the emulator could not: JSON state echoes were ignored, so every
  light command timed out. Fixed in v0.2.1.
- **Feedback cover with one shared endstop works** for a bench sim but makes
  the reported position meaningless; production nodes need two endstops.
