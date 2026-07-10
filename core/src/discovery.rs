//! HA MQTT Discovery ingestion. This module owns ALL the payload ugliness
//! (docs/mqtt-conventions.md): abbreviation expansion, validation, quarantine.
//! Nothing downstream ever sees an HA payload.

use crate::model::*;
use serde_json::{Map, Value as Json};

/// Components we fully support in v0.1.
const SUPPORTED: &[&str] = &["sensor", "binary_sensor", "switch", "light", "cover"];
/// Components we parse and store, but expose no commands for yet.
const DEFERRED: &[&str] = &[
    "climate", "lock", "number", "select", "button", "fan", "camera", "event",
];

/// Top-level and availability-item abbreviations (subset of HA's table
/// covering what ESPHome, Shelly, Tasmota, zigbee2mqtt and WLED emit).
const ABBREVIATIONS: &[(&str, &str)] = &[
    ("avty", "availability"),
    ("avty_mode", "availability_mode"),
    ("avty_t", "availability_topic"),
    ("bri_cmd_t", "brightness_command_topic"),
    ("bri_scl", "brightness_scale"),
    ("bri_stat_t", "brightness_state_topic"),
    ("cmd_t", "command_topic"),
    ("dev", "device"),
    ("dev_cla", "device_class"),
    ("ent_cat", "entity_category"),
    ("exp_aft", "expires_after"),
    ("ic", "icon"),
    ("json_attr_t", "json_attributes_topic"),
    ("obj_id", "object_id"),
    ("opt", "optimistic"),
    ("pl_avail", "payload_available"),
    ("pl_cls", "payload_close"),
    ("pl_not_avail", "payload_not_available"),
    ("pl_off", "payload_off"),
    ("pl_on", "payload_on"),
    ("pl_open", "payload_open"),
    ("pl_stop", "payload_stop"),
    ("pos_t", "position_topic"),
    ("pos_clsd", "position_closed"),
    ("pos_open", "position_open"),
    ("ret", "retain"),
    ("set_pos_t", "set_position_topic"),
    ("set_pos_tpl", "set_position_template"),
    ("stat_cla", "state_class"),
    ("stat_clsd", "state_closed"),
    ("stat_closing", "state_closing"),
    ("stat_off", "state_off"),
    ("stat_on", "state_on"),
    ("stat_open", "state_open"),
    ("stat_opening", "state_opening"),
    ("stat_stopped", "state_stopped"),
    ("stat_t", "state_topic"),
    ("t", "topic"),
    ("uniq_id", "unique_id"),
    ("unit_of_meas", "unit_of_measurement"),
    ("val_tpl", "value_template"),
];

/// Device-block abbreviations.
const DEVICE_ABBREVIATIONS: &[(&str, &str)] = &[
    ("cns", "connections"),
    ("cu", "configuration_url"),
    ("hw", "hw_version"),
    ("ids", "identifiers"),
    ("mdl", "model"),
    ("mdl_id", "model_id"),
    ("mf", "manufacturer"),
    ("sa", "suggested_area"),
    ("sw", "sw_version"),
];

fn expand_keys(map: Map<String, Json>, table: &[(&str, &str)]) -> Map<String, Json> {
    map.into_iter()
        .map(|(k, v)| {
            let full = table
                .iter()
                .find(|(a, _)| *a == k.as_str())
                .map(|(_, f)| f.to_string())
                .unwrap_or(k);
            (full, v)
        })
        .collect()
}

/// Expand abbreviations at the top level, inside the device block, and inside
/// availability list items.
fn expand(mut map: Map<String, Json>) -> Map<String, Json> {
    map = expand_keys(map, ABBREVIATIONS);
    if let Some(Json::Object(dev)) = map.remove("device") {
        map.insert(
            "device".into(),
            Json::Object(expand_keys(dev, DEVICE_ABBREVIATIONS)),
        );
    }
    if let Some(Json::Array(items)) = map.remove("availability") {
        let items = items
            .into_iter()
            .map(|i| match i {
                Json::Object(o) => Json::Object(expand_keys(o, ABBREVIATIONS)),
                other => other,
            })
            .collect();
        map.insert("availability".into(), Json::Array(items));
    }
    map
}

#[derive(Debug)]
pub enum Outcome {
    Upsert {
        device: Box<Device>,
        entity: Box<Entity>,
    },
    /// Empty retained payload = deletion, per HA convention. The registry
    /// resolves which entity via its config-topic index.
    Remove,
    Quarantine {
        reason: String,
    },
    /// Not a discovery config topic at all.
    NotDiscovery,
}

/// Parse `homeassistant/<component>/[<node_id>/]<object_id>/config`.
fn parse_topic(topic: &str) -> Option<(String, String)> {
    let rest = topic.strip_prefix("homeassistant/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    match parts.as_slice() {
        [component, object_id, "config"] => Some((component.to_string(), object_id.to_string())),
        [component, node_id, object_id, "config"] => {
            Some((component.to_string(), format!("{node_id}_{object_id}")))
        }
        _ => None,
    }
}

pub fn handle(topic: &str, payload: &[u8]) -> Outcome {
    let Some((component, fallback_object)) = parse_topic(topic) else {
        return Outcome::NotDiscovery;
    };

    if payload.is_empty() {
        return Outcome::Remove;
    }

    let deferred = DEFERRED.contains(&component.as_str());
    if !SUPPORTED.contains(&component.as_str()) && !deferred {
        return Outcome::Quarantine {
            reason: format!("unknown component '{component}'"),
        };
    }

    let json: Json = match serde_json::from_slice(payload) {
        Ok(j) => j,
        Err(e) => {
            return Outcome::Quarantine {
                reason: format!("unparseable JSON: {e}"),
            };
        }
    };
    let Json::Object(map) = json else {
        return Outcome::Quarantine {
            reason: "payload is not a JSON object".into(),
        };
    };
    let map = expand(map);

    let str_of = |k: &str| map.get(k).and_then(Json::as_str).map(str::to_string);

    // unique_id is required; without it there is no stable identity (spec rule).
    let Some(unique_id) = str_of("unique_id") else {
        return Outcome::Quarantine {
            reason: "missing unique_id".into(),
        };
    };

    let template = match str_of("value_template") {
        None => None,
        Some(t) => match Template::parse(&t) {
            Some(parsed) => Some(parsed),
            None => {
                return Outcome::Quarantine {
                    reason: format!(
                        "unsupported value_template '{t}' (only '{{{{ value }}}}' and '{{{{ value_json.<path> }}}}' are evaluated)"
                    ),
                };
            }
        },
    };

    let kind = match component.as_str() {
        "sensor" => EntityKind::Sensor,
        "binary_sensor" => EntityKind::BinarySensor,
        "switch" => EntityKind::Switch,
        "light" => EntityKind::Light,
        "cover" => EntityKind::Cover,
        other => EntityKind::Deferred(other.to_string()),
    };

    // Device block; a device-less entity gets a standalone synthesized device.
    let device = match map.get("device").and_then(Json::as_object) {
        Some(dev) => {
            let identifiers: Vec<String> = match dev.get("identifiers") {
                Some(Json::Array(a)) => a
                    .iter()
                    .filter_map(Json::as_str)
                    .map(str::to_string)
                    .collect(),
                Some(Json::String(s)) => vec![s.clone()],
                _ => vec![],
            };
            let connections: Vec<String> = match dev.get("connections") {
                Some(Json::Array(a)) => a
                    .iter()
                    .filter_map(|pair| {
                        let p = pair.as_array()?;
                        Some(format!("{}_{}", p.first()?.as_str()?, p.get(1)?.as_str()?))
                    })
                    .collect(),
                _ => vec![],
            };
            let key = identifiers.first().or(connections.first());
            let Some(key) = key else {
                return Outcome::Quarantine {
                    reason: "device block has neither identifiers nor connections".into(),
                };
            };
            let dstr = |k: &str| dev.get(k).and_then(Json::as_str).map(str::to_string);
            Device {
                id: sanitize_id(key),
                name: dstr("name").unwrap_or_else(|| key.clone()),
                manufacturer: dstr("manufacturer"),
                model: dstr("model"),
                sw_version: dstr("sw_version"),
                hw_version: dstr("hw_version"),
                identifiers,
            }
        }
        None => Device {
            id: sanitize_id(&format!("{unique_id}_dev")),
            name: str_of("name").unwrap_or_else(|| unique_id.clone()),
            manufacturer: None,
            model: None,
            sw_version: None,
            hw_version: None,
            identifiers: vec![],
        },
    };

    // Availability: single topic or list; defaults per HA ("online"/"offline").
    let mut availability = Vec::new();
    if let Some(t) = str_of("availability_topic") {
        availability.push(Availability {
            topic: t,
            payload_available: str_of("payload_available").unwrap_or_else(|| "online".into()),
            payload_not_available: str_of("payload_not_available")
                .unwrap_or_else(|| "offline".into()),
        });
    }
    if let Some(Json::Array(items)) = map.get("availability") {
        for item in items {
            if let Some(o) = item.as_object()
                && let Some(t) = o.get("topic").and_then(Json::as_str)
            {
                availability.push(Availability {
                    topic: t.to_string(),
                    payload_available: o
                        .get("payload_available")
                        .and_then(Json::as_str)
                        .unwrap_or("online")
                        .to_string(),
                    payload_not_available: o
                        .get("payload_not_available")
                        .and_then(Json::as_str)
                        .unwrap_or("offline")
                        .to_string(),
                });
            }
        }
    }

    let payload_on = str_of("payload_on").unwrap_or_else(|| "ON".into());
    let payload_off = str_of("payload_off").unwrap_or_else(|| "OFF".into());
    let command_topic = str_of("command_topic");

    let command = match (&kind, command_topic) {
        (EntityKind::Switch, Some(ct)) => Some(CommandCfg::Switch {
            command_topic: ct,
            payload_on: payload_on.clone(),
            payload_off: payload_off.clone(),
        }),
        (EntityKind::Light, Some(ct)) => {
            if str_of("schema").as_deref() == Some("json") {
                Some(CommandCfg::Light(LightCmd::Json {
                    command_topic: ct,
                    brightness: map
                        .get("brightness")
                        .and_then(Json::as_bool)
                        .unwrap_or(false),
                }))
            } else if str_of("schema").is_none() {
                Some(CommandCfg::Light(LightCmd::Basic {
                    command_topic: ct,
                    payload_on: payload_on.clone(),
                    payload_off: payload_off.clone(),
                    brightness_command_topic: str_of("brightness_command_topic"),
                    brightness_scale: map
                        .get("brightness_scale")
                        .and_then(Json::as_u64)
                        .unwrap_or(255) as u32,
                }))
            } else {
                return Outcome::Quarantine {
                    reason: format!(
                        "unsupported light schema '{}'",
                        str_of("schema").unwrap_or_default()
                    ),
                };
            }
        }
        (EntityKind::Cover, Some(ct)) => Some(CommandCfg::Cover {
            command_topic: ct,
            payload_open: str_of("payload_open").unwrap_or_else(|| "OPEN".into()),
            payload_close: str_of("payload_close").unwrap_or_else(|| "CLOSE".into()),
            payload_stop: str_of("payload_stop").unwrap_or_else(|| "STOP".into()),
            set_position_topic: str_of("set_position_topic"),
        }),
        _ => None,
    };

    let cover_states = matches!(kind, EntityKind::Cover).then(|| CoverStates {
        open: str_of("state_open").unwrap_or_else(|| "open".into()),
        opening: str_of("state_opening").unwrap_or_else(|| "opening".into()),
        closed: str_of("state_closed").unwrap_or_else(|| "closed".into()),
        closing: str_of("state_closing").unwrap_or_else(|| "closing".into()),
        stopped: str_of("state_stopped"),
    });

    let device_class = str_of("device_class");
    let entity = Entity {
        id: sanitize_id(&unique_id),
        unique_id,
        device_id: device.id.clone(),
        subsystem: suggest_subsystem(
            device_class.as_deref(),
            str_of("unit_of_measurement").as_deref(),
        ),
        kind: kind.clone(),
        name: str_of("name").unwrap_or_else(|| fallback_object.clone()),
        unit: str_of("unit_of_measurement"),
        device_class,
        criticality: Criticality::Info,
        state_topic: str_of("state_topic"),
        template,
        state_on: str_of("state_on").or(match kind {
            EntityKind::Switch | EntityKind::Light => Some(payload_on),
            _ => None,
        }),
        state_off: str_of("state_off").or(match kind {
            EntityKind::Switch | EntityKind::Light => Some(payload_off),
            _ => None,
        }),
        cover_states,
        brightness_state_topic: str_of("brightness_state_topic"),
        availability,
        command,
        optimistic: map
            .get("optimistic")
            .and_then(Json::as_bool)
            .unwrap_or(false),
        state: None,
        available: None,
        attributes: Default::default(),
    };

    Outcome::Upsert {
        device: Box::new(device),
        entity: Box::new(entity),
    }
}

/// First-pass subsystem suggestion from HA vocabulary. User-overridable.
fn suggest_subsystem(device_class: Option<&str>, unit: Option<&str>) -> Option<String> {
    let s = match device_class {
        Some("battery" | "current" | "voltage" | "power" | "energy" | "power_factor") => "power",
        Some("temperature" | "humidity") => "climate",
        Some("door" | "garage_door" | "lock" | "window") => "misc",
        Some("moisture" | "water") => "plumbing",
        _ => match unit {
            Some("V" | "A" | "W" | "kWh" | "Wh") => "power",
            Some("°C" | "°F" | "%") => "climate",
            _ => return None,
        },
    };
    Some(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A realistic abbreviated payload, ESPHome/Shelly style.
    const SHELLY_LIKE: &str = r#"{
        "name": "Channel A power",
        "uniq_id": "shelly3em-aabb-power-a",
        "stat_t": "shellies/em/0/power",
        "unit_of_meas": "W",
        "dev_cla": "power",
        "avty_t": "shellies/em/online",
        "pl_avail": "true",
        "pl_not_avail": "false",
        "dev": {"ids": ["shelly3em-aabb"], "mf": "Shelly", "mdl": "3EM", "name": "Shelly 3EM"}
    }"#;

    #[test]
    fn ingests_abbreviated_sensor() {
        let out = handle(
            "homeassistant/sensor/shelly3em-aabb/power-a/config",
            SHELLY_LIKE.as_bytes(),
        );
        let Outcome::Upsert { device, entity } = out else {
            panic!("expected upsert, got {out:?}")
        };
        assert_eq!(device.id, "shelly3em-aabb");
        assert_eq!(device.manufacturer.as_deref(), Some("Shelly"));
        assert_eq!(entity.id, "shelly3em-aabb-power-a");
        assert_eq!(entity.kind, EntityKind::Sensor);
        assert_eq!(entity.state_topic.as_deref(), Some("shellies/em/0/power"));
        assert_eq!(entity.unit.as_deref(), Some("W"));
        assert_eq!(entity.subsystem.as_deref(), Some("power"));
        assert_eq!(entity.availability.len(), 1);
        assert_eq!(entity.availability[0].payload_available, "true");
    }

    #[test]
    fn missing_unique_id_quarantines() {
        let out = handle(
            "homeassistant/sensor/x/config",
            br#"{"name":"x","stat_t":"a/b"}"#,
        );
        assert!(matches!(out, Outcome::Quarantine { reason } if reason.contains("unique_id")));
    }

    #[test]
    fn unknown_component_quarantines_and_junk_ignored() {
        let out = handle("homeassistant/vacuum/x/config", br#"{"uniq_id":"v1"}"#);
        assert!(matches!(out, Outcome::Quarantine { .. }));
        assert!(matches!(
            handle("shellies/em/0/power", b"42"),
            Outcome::NotDiscovery
        ));
    }

    #[test]
    fn deferred_component_upserts_without_commands() {
        let out = handle(
            "homeassistant/climate/x/config",
            br#"{"uniq_id":"heater1","name":"Heater","dev":{"ids":["h1"]}}"#,
        );
        let Outcome::Upsert { entity, .. } = out else {
            panic!()
        };
        assert_eq!(entity.kind, EntityKind::Deferred("climate".into()));
        assert!(entity.command.is_none());
    }

    #[test]
    fn unsupported_template_quarantines() {
        let out = handle(
            "homeassistant/sensor/x/config",
            br#"{"uniq_id":"s1","stat_t":"a/b","val_tpl":"{{ value_json.x | round(1) }}"}"#,
        );
        assert!(matches!(out, Outcome::Quarantine { reason } if reason.contains("value_template")));
    }

    #[test]
    fn empty_payload_removes() {
        assert!(matches!(
            handle("homeassistant/sensor/x/config", b""),
            Outcome::Remove
        ));
    }

    #[test]
    fn switch_command_mapping() {
        let out = handle(
            "homeassistant/switch/x/config",
            br#"{"uniq_id":"sw1","stat_t":"n/sw/state","cmd_t":"n/sw/cmd","dev":{"ids":["n1"]}}"#,
        );
        let Outcome::Upsert { entity, .. } = out else {
            panic!()
        };
        assert_eq!(
            entity.command,
            Some(CommandCfg::Switch {
                command_topic: "n/sw/cmd".into(),
                payload_on: "ON".into(),
                payload_off: "OFF".into()
            })
        );
        // Switch state matching falls back to payload_on/off.
        assert_eq!(entity.state_on.as_deref(), Some("ON"));
    }
}
