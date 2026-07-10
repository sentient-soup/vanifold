//! Virtual MQTT devices for integration-testing a vanifold hub.
//!
//! Connects to a real broker, announces a small van's worth of devices via HA
//! MQTT discovery, simulates plausible physics (solar curve, fridge cycling,
//! tank levels, battery drift), answers commands with realistic echoes
//! (including cover travel time), periodically drops one device offline to
//! exercise availability, and publishes two intentionally-broken discovery
//! payloads to exercise quarantine.
//!
//! Usage:
//!   vanifold-emulator [--broker HOST] [--port N] [--username U] [--password P]

use rumqttc::v5::mqttbytes::QoS;
use rumqttc::v5::mqttbytes::v5::{LastWill, Packet};
use rumqttc::v5::{AsyncClient, Event, MqttOptions};
use serde_json::json;
use std::time::Duration;

const TICK: Duration = Duration::from_secs(2);

struct Args {
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
}

fn parse_args() -> Args {
    let mut args = Args {
        host: "localhost".into(),
        port: 1883,
        username: None,
        password: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut val = || it.next().unwrap_or_default();
        match flag.as_str() {
            "--broker" => args.host = val(),
            "--port" => args.port = val().parse().expect("--port must be a number"),
            "--username" => args.username = Some(val()),
            "--password" => args.password = Some(val()),
            other => {
                eprintln!("unknown flag {other}");
                std::process::exit(2);
            }
        }
    }
    args
}

/// One announced entity: discovery topic + payload, plus optional command
/// subscription.
struct Announce {
    config_topic: String,
    payload: serde_json::Value,
}

fn device(id: &str, name: &str, model: &str) -> serde_json::Value {
    json!({ "identifiers": [id], "name": name, "manufacturer": "vanifold-emulator", "model": model })
}

fn announcements() -> Vec<Announce> {
    let mut list = Vec::new();
    let mut add = |component: &str, object: &str, payload: serde_json::Value| {
        list.push(Announce {
            config_topic: format!("homeassistant/{component}/emu/{object}/config"),
            payload,
        })
    };

    // Emulated Shelly 3EM: three AC power channels + line voltage.
    let em = device("emu-3em", "Emulated 3EM", "3EM");
    for (ch, label) in [
        ("a", "Solar circuit"),
        ("b", "Fridge circuit"),
        ("c", "Misc circuit"),
    ] {
        add(
            "sensor",
            &format!("3em-power-{ch}"),
            json!({
                "unique_id": format!("emu-3em-power-{ch}"), "name": format!("{label} power"),
                "state_topic": format!("emu/3em/power_{ch}"), "unit_of_measurement": "W",
                "device_class": "power", "availability_topic": "emu/3em/status", "device": em
            }),
        );
    }
    add(
        "sensor",
        "3em-voltage",
        json!({
            "unique_id": "emu-3em-voltage", "name": "AC voltage",
            "state_topic": "emu/3em/voltage", "unit_of_measurement": "V",
            "device_class": "voltage", "availability_topic": "emu/3em/status", "device": em
        }),
    );

    // Emulated BMS: retained states (exercises quality=retained after a hub restart).
    let bms = device("emu-bms", "Emulated house BMS", "BMS");
    for (obj, name, unit, class) in [
        ("bms-voltage", "Battery voltage", "V", "voltage"),
        ("bms-soc", "State of charge", "%", "battery"),
        ("bms-temp", "Battery temp", "°C", "temperature"),
    ] {
        add(
            "sensor",
            obj,
            json!({
                "unique_id": format!("emu-{obj}"), "name": name,
                "state_topic": format!("emu/bms/{obj}"), "unit_of_measurement": unit,
                "device_class": class, "availability_topic": "emu/bms/status", "device": bms
            }),
        );
    }

    // Plumbing node: tanks + commandable pump (uses abbreviated keys on
    // purpose: exercises the expansion path).
    let plumbing = device("emu-plumbing", "Emulated plumbing node", "ESP32");
    add(
        "sensor",
        "fresh-level",
        json!({
            "uniq_id": "emu-fresh-level", "name": "Fresh water",
            "stat_t": "emu/plumbing/fresh", "unit_of_meas": "%",
            "avty_t": "emu/plumbing/status", "dev": plumbing
        }),
    );
    add(
        "sensor",
        "grey-level",
        json!({
            "uniq_id": "emu-grey-level", "name": "Grey water",
            "stat_t": "emu/plumbing/grey", "unit_of_meas": "%",
            "avty_t": "emu/plumbing/status", "dev": plumbing
        }),
    );
    add(
        "switch",
        "pump",
        json!({
            "uniq_id": "emu-pump", "name": "Water pump",
            "stat_t": "emu/plumbing/pump/state", "cmd_t": "emu/plumbing/pump/set",
            "avty_t": "emu/plumbing/status", "dev": plumbing
        }),
    );

    // Cabin climate: this device blacks out periodically.
    let climate = device("emu-climate", "Emulated cabin sensor", "Zigbee");
    for (obj, name, unit, class, topic) in [
        (
            "cabin-temp",
            "Cabin temp",
            "°C",
            "temperature",
            "emu/climate/temp",
        ),
        (
            "cabin-humidity",
            "Cabin humidity",
            "%",
            "humidity",
            "emu/climate/humidity",
        ),
    ] {
        add(
            "sensor",
            obj,
            json!({
                "unique_id": format!("emu-{obj}"), "name": name,
                "state_topic": topic, "unit_of_measurement": unit, "device_class": class,
                "availability_topic": "emu/climate/status", "device": climate
            }),
        );
    }

    // Rear entry: cover with real travel time, plus a door contact.
    let rear = device("emu-rear", "Emulated rear entry node", "ESP32");
    add(
        "cover",
        "rear-door",
        json!({
            "unique_id": "emu-rear-door", "name": "Rear entry",
            "state_topic": "emu/rear/door/state", "command_topic": "emu/rear/door/set",
            "availability_topic": "emu/rear/status", "device": rear, "device_class": "door"
        }),
    );
    add(
        "binary_sensor",
        "cab-door",
        json!({
            "unique_id": "emu-cab-door", "name": "Cab door", "device_class": "door",
            "state_topic": "emu/rear/cab_door", "availability_topic": "emu/rear/status", "device": rear
        }),
    );

    // Dimmable light, HA default schema.
    let light = device("emu-light", "Emulated galley light", "ESP32");
    add(
        "light",
        "galley",
        json!({
            "unique_id": "emu-galley-light", "name": "Galley light",
            "state_topic": "emu/light/state", "command_topic": "emu/light/set",
            "brightness_state_topic": "emu/light/brightness/state",
            "brightness_command_topic": "emu/light/brightness/set",
            "brightness_scale": 255,
            "availability_topic": "emu/light/status", "device": light
        }),
    );

    // Broken on purpose: missing unique_id, and an unsupported component.
    // Both must land in quarantine, never crash anything.
    add(
        "sensor",
        "junk-no-id",
        json!({ "name": "Junk sensor", "state_topic": "emu/junk" }),
    );
    add(
        "vacuum",
        "junk-vacuum",
        json!({ "unique_id": "emu-vacuum", "name": "Junk vacuum", "state_topic": "emu/vacuum" }),
    );

    list
}

const AVAILABILITY_TOPICS: &[&str] = &[
    "emu/3em/status",
    "emu/bms/status",
    "emu/plumbing/status",
    "emu/climate/status",
    "emu/rear/status",
    "emu/light/status",
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum CoverState {
    Open,
    Opening { until_tick: u64 },
    Closed,
    Closing { until_tick: u64 },
    Stopped,
}

impl CoverState {
    fn name(&self) -> &'static str {
        match self {
            CoverState::Open => "open",
            CoverState::Opening { .. } => "opening",
            CoverState::Closed => "closed",
            CoverState::Closing { .. } => "closing",
            CoverState::Stopped => "stopped",
        }
    }
}

/// Simulation state. Deterministic pseudo-noise from overlapping sines: no
/// RNG needed, and runs reproduce.
struct Sim {
    tick: u64,
    pump_on: bool,
    light_on: bool,
    brightness: u32,
    cover: CoverState,
    fresh: f64,
    grey: f64,
    climate_online: bool,
}

const COVER_TRAVEL_TICKS: u64 = 3; // ~6s of winch travel

impl Sim {
    fn new() -> Self {
        Sim {
            tick: 0,
            pump_on: false,
            light_on: true,
            brightness: 180,
            cover: CoverState::Closed,
            fresh: 63.0,
            grey: 41.0,
            climate_online: true,
        }
    }

    fn t(&self) -> f64 {
        self.tick as f64 * TICK.as_secs_f64()
    }

    fn noise(&self, freq: f64, amp: f64) -> f64 {
        (self.t() * freq).sin() * amp + (self.t() * freq * 3.7).sin() * amp * 0.3
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).init();
    let args = parse_args();

    let mut options = MqttOptions::new("vanifold-emulator", args.host.clone(), args.port);
    options.set_keep_alive(Duration::from_secs(15));
    if let (Some(u), Some(p)) = (&args.username, &args.password) {
        options.set_credentials(u.clone(), p.clone());
    }
    options.set_last_will(LastWill::new(
        "emu/bridge/status",
        "offline",
        QoS::AtLeastOnce,
        true,
        None,
    ));

    let (client, mut eventloop) = AsyncClient::new(options, 64);
    let mut sim = Sim::new();
    let mut ticker = tokio::time::interval(TICK);

    loop {
        tokio::select! {
            event = eventloop.poll() => match event {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    tracing::info!(host = %args.host, port = args.port, "connected, announcing fleet");
                    announce(&client).await;
                }
                Ok(Event::Incoming(Packet::Publish(p))) => {
                    let topic = String::from_utf8_lossy(&p.topic).into_owned();
                    let payload = String::from_utf8_lossy(&p.payload).into_owned();
                    handle_command(&client, &mut sim, &topic, &payload).await;
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(%e, "mqtt error, retrying in 2s");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            },
            _ = ticker.tick() => {
                sim.tick += 1;
                step(&client, &mut sim).await;
            }
        }
    }
}

async fn announce(client: &AsyncClient) {
    for topic in [
        "emu/plumbing/pump/set",
        "emu/rear/door/set",
        "emu/light/set",
        "emu/light/brightness/set",
    ] {
        client.subscribe(topic, QoS::AtLeastOnce).await.ok();
    }
    for a in announcements() {
        client
            .publish(
                a.config_topic,
                QoS::AtLeastOnce,
                true,
                a.payload.to_string(),
            )
            .await
            .ok();
    }
    client
        .publish("emu/bridge/status", QoS::AtLeastOnce, true, "online")
        .await
        .ok();
    for t in AVAILABILITY_TOPICS {
        client
            .publish(*t, QoS::AtLeastOnce, true, "online")
            .await
            .ok();
    }
}

async fn handle_command(client: &AsyncClient, sim: &mut Sim, topic: &str, payload: &str) {
    tracing::info!(topic, payload, "command received");
    // Small echo latency, like real firmware.
    tokio::time::sleep(Duration::from_millis(150)).await;
    match topic {
        "emu/plumbing/pump/set" => {
            sim.pump_on = payload == "ON";
            pub_now(
                client,
                "emu/plumbing/pump/state",
                if sim.pump_on { "ON" } else { "OFF" },
                false,
            )
            .await;
        }
        "emu/light/set" => {
            sim.light_on = payload == "ON";
            pub_now(
                client,
                "emu/light/state",
                if sim.light_on { "ON" } else { "OFF" },
                false,
            )
            .await;
        }
        "emu/light/brightness/set" => {
            if let Ok(b) = payload.trim().parse() {
                sim.brightness = b;
                sim.light_on = b > 0;
                pub_now(
                    client,
                    "emu/light/brightness/state",
                    &sim.brightness.to_string(),
                    false,
                )
                .await;
                pub_now(
                    client,
                    "emu/light/state",
                    if sim.light_on { "ON" } else { "OFF" },
                    false,
                )
                .await;
            }
        }
        "emu/rear/door/set" => {
            sim.cover = match payload {
                "OPEN" => CoverState::Opening {
                    until_tick: sim.tick + COVER_TRAVEL_TICKS,
                },
                "CLOSE" => CoverState::Closing {
                    until_tick: sim.tick + COVER_TRAVEL_TICKS,
                },
                "STOP" => CoverState::Stopped,
                _ => sim.cover,
            };
            pub_now(client, "emu/rear/door/state", sim.cover.name(), false).await;
        }
        _ => {}
    }
}

async fn pub_now(client: &AsyncClient, topic: &str, payload: &str, retain: bool) {
    client
        .publish(
            topic.to_owned(),
            QoS::AtLeastOnce,
            retain,
            payload.to_owned(),
        )
        .await
        .ok();
}

async fn step(client: &AsyncClient, sim: &mut Sim) {
    let t = sim.t();

    // Cover travel completes; state republishes every tick so the hub always
    // has one (fresh subscribers would otherwise see nothing until a command).
    match sim.cover {
        CoverState::Opening { until_tick } if sim.tick >= until_tick => {
            sim.cover = CoverState::Open;
        }
        CoverState::Closing { until_tick } if sim.tick >= until_tick => {
            sim.cover = CoverState::Closed;
        }
        _ => {}
    }
    pub_now(client, "emu/rear/door/state", sim.cover.name(), false).await;

    // Climate device blackout: offline for the last 20s of every 2 minutes.
    let online = t % 120.0 < 100.0;
    if online != sim.climate_online {
        sim.climate_online = online;
        tracing::info!(online, "climate device availability change");
        pub_now(
            client,
            "emu/climate/status",
            if online { "online" } else { "offline" },
            true,
        )
        .await;
    }

    // Tanks: pump drains fresh into grey.
    let draw = if sim.pump_on { 0.35 } else { 0.01 };
    sim.fresh = (sim.fresh - draw).max(0.0);
    sim.grey = (sim.grey + draw * 0.8).min(100.0);

    // 3EM channels: solar day-curve, fridge duty cycle, misc noise.
    let solar = (320.0 + (t / 60.0).sin() * 260.0 + sim.noise(0.9, 18.0)).max(0.0);
    let fridge = if t % 150.0 < 60.0 {
        47.0 + sim.noise(1.3, 2.0)
    } else {
        0.0
    };
    let misc = 22.0 + sim.noise(0.4, 9.0).abs();
    pub_now(client, "emu/3em/power_a", &format!("{solar:.1}"), false).await;
    pub_now(client, "emu/3em/power_b", &format!("{fridge:.1}"), false).await;
    pub_now(client, "emu/3em/power_c", &format!("{misc:.1}"), false).await;
    pub_now(
        client,
        "emu/3em/voltage",
        &format!("{:.1}", 231.0 + sim.noise(0.2, 1.4)),
        false,
    )
    .await;

    // BMS (retained: exercises quality=retained after hub restarts).
    let voltage = 52.2 + (t / 300.0).sin() * 1.4;
    let soc = 86.0 + (t / 500.0).sin() * 6.0;
    pub_now(
        client,
        "emu/bms/bms-voltage",
        &format!("{voltage:.2}"),
        true,
    )
    .await;
    pub_now(client, "emu/bms/bms-soc", &format!("{soc:.0}"), true).await;
    pub_now(
        client,
        "emu/bms/bms-temp",
        &format!("{:.1}", 18.0 + (t / 120.0).sin() * 2.0),
        true,
    )
    .await;

    pub_now(
        client,
        "emu/plumbing/fresh",
        &format!("{:.1}", sim.fresh),
        false,
    )
    .await;
    pub_now(
        client,
        "emu/plumbing/grey",
        &format!("{:.1}", sim.grey),
        false,
    )
    .await;
    pub_now(
        client,
        "emu/plumbing/pump/state",
        if sim.pump_on { "ON" } else { "OFF" },
        false,
    )
    .await;

    if sim.climate_online {
        pub_now(
            client,
            "emu/climate/temp",
            &format!("{:.1}", 21.5 + (t / 90.0).sin() * 1.2),
            false,
        )
        .await;
        pub_now(
            client,
            "emu/climate/humidity",
            &format!("{:.0}", 45.0 + (t / 70.0).sin() * 6.0),
            false,
        )
        .await;
    }

    // Cab door swings every ~90s.
    pub_now(
        client,
        "emu/rear/cab_door",
        if t % 180.0 < 90.0 { "OFF" } else { "ON" },
        false,
    )
    .await;

    pub_now(
        client,
        "emu/light/state",
        if sim.light_on { "ON" } else { "OFF" },
        false,
    )
    .await;
    pub_now(
        client,
        "emu/light/brightness/state",
        &sim.brightness.to_string(),
        false,
    )
    .await;

    if sim.tick.is_multiple_of(15) {
        tracing::info!(
            tick = sim.tick,
            solar = format!("{solar:.0}W"),
            fresh = format!("{:.0}%", sim.fresh),
            "sim running"
        );
    }
}
