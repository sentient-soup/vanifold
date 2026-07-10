//! Internal entity model, per docs/entity-model.md. Nothing in here knows what
//! an HA discovery payload looks like; that is discovery.rs's quarantine zone.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub type EntityId = String;
pub type DeviceId = String;

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before unix epoch")
        .as_millis() as u64
}

/// Sanitize an external id (unique_id, device identifier) into a stable id
/// that survives becoming a topic segment or URL path later.
pub fn sanitize_id(raw: &str) -> String {
    let mut out: String = raw
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        out.push('_');
    }
    out
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Text(String),
}

impl Value {
    /// Only numeric and on/off states are historized (docs/entity-model.md).
    pub fn history_point(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Value::Text(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Live,
    Retained,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub value: Value,
    pub updated_at: u64,
    /// True while the latest value came from broker retention (age unknown).
    pub retained: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Sensor,
    BinarySensor,
    Switch,
    Light,
    Cover,
    /// Recognized HA component we parse and store but expose no commands for
    /// yet (climate, lock, number, ...). Carries the component name.
    Deferred(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Criticality {
    #[default]
    Info,
    Comfort,
    Safety,
}

/// The supported subset of HA value_template: `{{ value }}` and
/// `{{ value_json.a.b }}`. Anything else quarantines the entity (spec rule).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Template {
    Identity,
    JsonPath(Vec<String>),
}

impl Template {
    pub fn parse(s: &str) -> Option<Template> {
        let inner = s.trim().strip_prefix("{{")?.strip_suffix("}}")?.trim();
        if inner == "value" {
            return Some(Template::Identity);
        }
        let path = inner.strip_prefix("value_json.")?;
        if path.is_empty() {
            return None;
        }
        let segments: Vec<String> = path.split('.').map(str::to_string).collect();
        if segments
            .iter()
            .any(|s| s.is_empty() || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        {
            return None;
        }
        Some(Template::JsonPath(segments))
    }

    /// Extract the raw string this template selects from a payload.
    pub fn apply(&self, payload: &str) -> Option<String> {
        match self {
            Template::Identity => Some(payload.to_string()),
            Template::JsonPath(path) => {
                let mut v: &serde_json::Value = &serde_json::from_str(payload).ok()?;
                for seg in path {
                    v = v.get(seg)?;
                }
                Some(match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Availability {
    pub topic: String,
    pub payload_available: String,
    pub payload_not_available: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverStates {
    pub open: String,
    pub opening: String,
    pub closed: String,
    pub closing: String,
    pub stopped: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LightCmd {
    /// HA "default" schema: plain payloads, separate brightness topic.
    Basic {
        command_topic: String,
        payload_on: String,
        payload_off: String,
        brightness_command_topic: Option<String>,
        brightness_scale: u32,
    },
    /// HA "json" schema: one topic, JSON body.
    Json {
        command_topic: String,
        brightness: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommandCfg {
    Switch {
        command_topic: String,
        payload_on: String,
        payload_off: String,
    },
    Light(LightCmd),
    Cover {
        command_topic: String,
        payload_open: String,
        payload_close: String,
        payload_stop: String,
        set_position_topic: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub unique_id: String,
    pub device_id: DeviceId,
    pub kind: EntityKind,
    pub name: String,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub device_class: Option<String>,
    #[serde(default)]
    pub criticality: Criticality,
    /// Auto-suggested from device_class/unit; user-overridable later.
    #[serde(default)]
    pub subsystem: Option<String>,

    #[serde(default)]
    pub state_topic: Option<String>,
    #[serde(default)]
    pub template: Option<Template>,
    /// State payloads matching these mark on/off (binary_sensor, switch, light basic).
    #[serde(default)]
    pub state_on: Option<String>,
    #[serde(default)]
    pub state_off: Option<String>,
    #[serde(default)]
    pub cover_states: Option<CoverStates>,
    #[serde(default)]
    pub brightness_state_topic: Option<String>,
    #[serde(default)]
    pub availability: Vec<Availability>,
    #[serde(default)]
    pub command: Option<CommandCfg>,
    #[serde(default)]
    pub optimistic: bool,

    #[serde(default)]
    pub state: Option<State>,
    /// None until an availability topic has spoken.
    #[serde(default)]
    pub available: Option<bool>,
    /// Extras the UI may want (e.g. brightness); an escape hatch, not a design surface.
    #[serde(default)]
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

impl Entity {
    pub fn quality(&self, now: u64, stale_after_ms: u64) -> Quality {
        if self.available == Some(false) {
            return Quality::Unavailable;
        }
        match &self.state {
            None => Quality::Stale,
            Some(s) if s.retained => Quality::Retained,
            Some(s) if now.saturating_sub(s.updated_at) > stale_after_ms => Quality::Stale,
            Some(_) => Quality::Live,
        }
    }

    /// Interpret a raw state payload per kind. None = payload did not parse
    /// into a state for this entity (logged upstream, never a crash).
    pub fn interpret(&self, payload: &str) -> Option<Value> {
        let raw = match &self.template {
            Some(t) => t.apply(payload)?,
            None => payload.to_string(),
        };
        let raw = raw.trim();
        match &self.kind {
            EntityKind::Sensor | EntityKind::Deferred(_) => Some(
                raw.parse::<f64>()
                    .map(Value::Number)
                    .unwrap_or_else(|_| Value::Text(raw.to_string())),
            ),
            EntityKind::BinarySensor | EntityKind::Switch | EntityKind::Light => {
                let on = self.state_on.as_deref().unwrap_or("ON");
                let off = self.state_off.as_deref().unwrap_or("OFF");
                if raw == on {
                    Some(Value::Bool(true))
                } else if raw == off {
                    Some(Value::Bool(false))
                } else {
                    None
                }
            }
            EntityKind::Cover => {
                let cs = self.cover_states.as_ref()?;
                let name = if raw == cs.open {
                    "open"
                } else if raw == cs.opening {
                    "opening"
                } else if raw == cs.closed {
                    "closed"
                } else if raw == cs.closing {
                    "closing"
                } else if Some(raw) == cs.stopped.as_deref() {
                    "stopped"
                } else {
                    return None;
                };
                Some(Value::Text(name.to_string()))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceId,
    pub name: String,
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub sw_version: Option<String>,
    #[serde(default)]
    pub hw_version: Option<String>,
    #[serde(default)]
    pub identifiers: Vec<String>,
}

/// Commands accepted over the API. Kind-checked in registry::command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    TurnOn,
    TurnOff,
    SetBrightness { brightness: u8 },
    Open,
    Close,
    Stop,
    SetPosition { position: u8 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_parsing() {
        assert_eq!(Template::parse("{{ value }}"), Some(Template::Identity));
        assert_eq!(
            Template::parse("{{ value_json.temperature }}"),
            Some(Template::JsonPath(vec!["temperature".into()]))
        );
        assert_eq!(
            Template::parse("{{value_json.a.b}}"),
            Some(Template::JsonPath(vec!["a".into(), "b".into()]))
        );
        // Full Jinja is out of scope: reject, don't guess.
        assert_eq!(Template::parse("{{ value_json.x | round(1) }}"), None);
        assert_eq!(Template::parse("{% if value %}1{% endif %}"), None);
    }

    #[test]
    fn template_apply() {
        let t = Template::JsonPath(vec!["a".into(), "b".into()]);
        assert_eq!(t.apply(r#"{"a":{"b":21.5}}"#), Some("21.5".into()));
        assert_eq!(t.apply(r#"{"a":{"b":"on"}}"#), Some("on".into()));
        assert_eq!(t.apply(r#"{"a":{}}"#), None);
        assert_eq!(t.apply("not json"), None);
    }

    fn sensor() -> Entity {
        Entity {
            id: "t1".into(),
            unique_id: "t1".into(),
            device_id: "d1".into(),
            kind: EntityKind::Sensor,
            name: "t1".into(),
            unit: None,
            device_class: None,
            criticality: Criticality::Info,
            subsystem: None,
            state_topic: Some("x/state".into()),
            template: None,
            state_on: None,
            state_off: None,
            cover_states: None,
            brightness_state_topic: None,
            availability: vec![],
            command: None,
            optimistic: false,
            state: None,
            available: None,
            attributes: Default::default(),
        }
    }

    #[test]
    fn interpret_by_kind() {
        let s = sensor();
        assert_eq!(s.interpret("21.5"), Some(Value::Number(21.5)));
        assert_eq!(s.interpret("weird"), Some(Value::Text("weird".into())));

        let mut b = sensor();
        b.kind = EntityKind::BinarySensor;
        assert_eq!(b.interpret("ON"), Some(Value::Bool(true)));
        assert_eq!(b.interpret("OFF"), Some(Value::Bool(false)));
        assert_eq!(b.interpret("MAYBE"), None);

        let mut c = sensor();
        c.kind = EntityKind::Cover;
        c.cover_states = Some(CoverStates {
            open: "open".into(),
            opening: "opening".into(),
            closed: "closed".into(),
            closing: "closing".into(),
            stopped: None,
        });
        assert_eq!(c.interpret("opening"), Some(Value::Text("opening".into())));
        assert_eq!(c.interpret("nope"), None);
    }

    #[test]
    fn quality_ladder() {
        let now = now_ms();
        let mut e = sensor();
        assert_eq!(e.quality(now, 900_000), Quality::Stale); // never heard
        e.state = Some(State {
            value: Value::Number(1.0),
            updated_at: now,
            retained: true,
        });
        assert_eq!(e.quality(now, 900_000), Quality::Retained);
        e.state = Some(State {
            value: Value::Number(1.0),
            updated_at: now,
            retained: false,
        });
        assert_eq!(e.quality(now, 900_000), Quality::Live);
        e.state = Some(State {
            value: Value::Number(1.0),
            updated_at: now - 1_000_000,
            retained: false,
        });
        assert_eq!(e.quality(now, 900_000), Quality::Stale);
        e.available = Some(false);
        assert_eq!(e.quality(now, 900_000), Quality::Unavailable);
    }
}
