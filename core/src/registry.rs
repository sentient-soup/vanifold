//! In-memory entity registry: the single source of truth the API serves.
//! Mutations arrive from MQTT (discovery + state), persistence flows to the
//! store thread, changes broadcast to WebSocket subscribers.

use crate::discovery::{self, Outcome};
use crate::interlock;
use crate::model::*;
use crate::store::StoreMsg;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::mpsc::Sender as StoreTx;
use tokio::sync::broadcast;

/// Events broadcast to API subscribers (and mirrored to the store where relevant).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    EntityUpserted {
        entity: Box<Entity>,
    },
    EntityRemoved {
        entity_id: EntityId,
    },
    StateChanged {
        entity_id: EntityId,
        state: State,
    },
    AvailabilityChanged {
        entity_id: EntityId,
        available: bool,
    },
    AttributeChanged {
        entity_id: EntityId,
        key: String,
        value: serde_json::Value,
    },
}

/// User-editable entity metadata (PATCH /api/entities/{id}).
#[derive(Debug, serde::Deserialize)]
pub struct MetaPatch {
    pub name: Option<String>,
    /// Empty string clears the subsystem back to unassigned.
    pub subsystem: Option<String>,
    pub criticality: Option<Criticality>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuarantineItem {
    pub topic: String,
    pub payload: String,
    pub reason: String,
    pub ts: u64,
}

#[derive(Default)]
struct Inner {
    devices: HashMap<DeviceId, Device>,
    entities: HashMap<EntityId, Entity>,
    /// state topic -> entity ids listening on it
    by_state_topic: HashMap<String, Vec<EntityId>>,
    /// availability topic -> entity ids listening on it
    by_avail_topic: HashMap<String, Vec<EntityId>>,
    /// brightness state topic -> entity ids
    by_brightness_topic: HashMap<String, Vec<EntityId>>,
    /// discovery config topic -> entity id (for empty-payload removals)
    by_config_topic: HashMap<String, EntityId>,
    quarantine: HashMap<String, QuarantineItem>,
}

pub struct Registry {
    inner: RwLock<Inner>,
    events: broadcast::Sender<Event>,
    store: StoreTx<StoreMsg>,
}

fn index(map: &mut HashMap<String, Vec<EntityId>>, topic: &str, id: &str) {
    let ids = map.entry(topic.to_string()).or_default();
    if !ids.iter().any(|i| i == id) {
        ids.push(id.to_string());
    }
}

fn unindex(map: &mut HashMap<String, Vec<EntityId>>, id: &str) {
    map.retain(|_, ids| {
        ids.retain(|i| i != id);
        !ids.is_empty()
    });
}

impl Registry {
    pub fn new(store: StoreTx<StoreMsg>) -> Self {
        let (events, _) = broadcast::channel(256);
        Registry {
            inner: RwLock::new(Inner::default()),
            events,
            store,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    /// Seed from the store at startup (states come back as retained-quality
    /// last-known values only via broker retention, not from history).
    pub fn load(&self, devices: Vec<Device>, entities: Vec<Entity>) {
        let mut inner = self.inner.write().unwrap();
        for d in devices {
            inner.devices.insert(d.id.clone(), d);
        }
        for e in entities {
            if let Some(t) = &e.state_topic {
                let t = t.clone();
                index(&mut inner.by_state_topic, &t, &e.id);
            }
            for a in e.availability.clone() {
                index(&mut inner.by_avail_topic, &a.topic, &e.id);
            }
            if let Some(t) = e.brightness_state_topic.clone() {
                index(&mut inner.by_brightness_topic, &t, &e.id);
            }
            inner.entities.insert(e.id.clone(), e);
        }
    }

    /// Route one incoming MQTT publish. The broker subscription is `#`
    /// (docs/mqtt-conventions.md): config topics feed discovery, everything
    /// else is matched against registered state/availability topics.
    pub fn handle_publish(&self, topic: &str, payload: &[u8], retained: bool) {
        match discovery::handle(topic, payload) {
            Outcome::NotDiscovery => self.handle_state(topic, payload, retained),
            Outcome::Upsert { device, entity } => self.upsert(topic, *device, *entity),
            Outcome::Remove => self.remove_by_config_topic(topic),
            Outcome::Quarantine { reason } => {
                tracing::warn!(topic, %reason, "discovery payload quarantined");
                let item = QuarantineItem {
                    topic: topic.to_string(),
                    payload: String::from_utf8_lossy(payload).into_owned(),
                    reason,
                    ts: now_ms(),
                };
                self.store.send(StoreMsg::Quarantine(item.clone())).ok();
                self.inner
                    .write()
                    .unwrap()
                    .quarantine
                    .insert(topic.to_string(), item);
            }
        }
    }

    fn upsert(&self, config_topic: &str, device: Device, mut entity: Entity) {
        let mut inner = self.inner.write().unwrap();
        // Re-announcement keeps runtime state; identity is unique_id.
        if let Some(existing) = inner.entities.get(&entity.id) {
            entity.state = existing.state.clone();
            entity.available = existing.available;
            entity.attributes = existing.attributes.clone();
            entity.criticality = existing.criticality; // user-set, survives re-announce
            entity.subsystem = existing.subsystem.clone().or(entity.subsystem);
            unindex(&mut inner.by_state_topic, &entity.id);
            unindex(&mut inner.by_avail_topic, &entity.id);
            unindex(&mut inner.by_brightness_topic, &entity.id);
        }
        if let Some(t) = &entity.state_topic {
            let t = t.clone();
            index(&mut inner.by_state_topic, &t, &entity.id);
        }
        for a in &entity.availability {
            let t = a.topic.clone();
            index(&mut inner.by_avail_topic, &t, &entity.id);
        }
        if let Some(t) = &entity.brightness_state_topic {
            let t = t.clone();
            index(&mut inner.by_brightness_topic, &t, &entity.id);
        }
        inner
            .by_config_topic
            .insert(config_topic.to_string(), entity.id.clone());
        inner.quarantine.remove(config_topic); // a good payload heals a quarantined topic
        inner.devices.insert(device.id.clone(), device.clone());
        inner.entities.insert(entity.id.clone(), entity.clone());
        drop(inner);

        tracing::info!(entity = %entity.id, device = %device.id, "entity upserted");
        self.store
            .send(StoreMsg::UpsertDevice(Box::new(device)))
            .ok();
        self.store
            .send(StoreMsg::UpsertEntity(Box::new(entity.clone())))
            .ok();
        self.store
            .send(StoreMsg::RemoveQuarantine(config_topic.to_string()))
            .ok();
        self.events
            .send(Event::EntityUpserted {
                entity: Box::new(entity),
            })
            .ok();
    }

    fn remove_by_config_topic(&self, config_topic: &str) {
        let mut inner = self.inner.write().unwrap();
        let Some(id) = inner.by_config_topic.remove(config_topic) else {
            return;
        };
        inner.entities.remove(&id);
        unindex(&mut inner.by_state_topic, &id);
        unindex(&mut inner.by_avail_topic, &id);
        unindex(&mut inner.by_brightness_topic, &id);
        drop(inner);
        tracing::info!(entity = %id, "entity removed (empty discovery payload)");
        self.store.send(StoreMsg::RemoveEntity(id.clone())).ok();
        self.events
            .send(Event::EntityRemoved { entity_id: id })
            .ok();
    }

    fn handle_state(&self, topic: &str, payload: &[u8], retained: bool) {
        let payload = String::from_utf8_lossy(payload);
        let mut inner = self.inner.write().unwrap();
        let mut events = Vec::new();
        let mut store_msgs = Vec::new();

        for id in inner.by_state_topic.get(topic).cloned().unwrap_or_default() {
            if let Some(e) = inner.entities.get_mut(&id) {
                match e.interpret(&payload) {
                    Some(value) => {
                        let state = State {
                            value,
                            updated_at: now_ms(),
                            retained,
                        };
                        e.state = Some(state.clone());
                        if let Some(v) = state.value.history_point() {
                            store_msgs.push(StoreMsg::Point {
                                entity_id: id.clone(),
                                ts: state.updated_at,
                                value: v,
                            });
                        }
                        events.push(Event::StateChanged {
                            entity_id: id,
                            state,
                        });
                    }
                    None => {
                        tracing::debug!(entity = %id, topic, %payload, "state payload did not interpret");
                    }
                }
            }
        }
        for id in inner.by_avail_topic.get(topic).cloned().unwrap_or_default() {
            if let Some(e) = inner.entities.get_mut(&id) {
                let avail = e.availability.iter().find_map(|a| {
                    if a.topic != topic {
                        return None;
                    }
                    if payload == a.payload_available {
                        Some(true)
                    } else if payload == a.payload_not_available {
                        Some(false)
                    } else {
                        None
                    }
                });
                if let Some(avail) = avail
                    && e.available != Some(avail)
                {
                    e.available = Some(avail);
                    events.push(Event::AvailabilityChanged {
                        entity_id: id,
                        available: avail,
                    });
                }
            }
        }
        for id in inner
            .by_brightness_topic
            .get(topic)
            .cloned()
            .unwrap_or_default()
        {
            if let Some(e) = inner.entities.get_mut(&id)
                && let Ok(b) = payload.trim().parse::<u64>()
            {
                let v = serde_json::json!(b);
                e.attributes.insert("brightness".into(), v.clone());
                events.push(Event::AttributeChanged {
                    entity_id: id,
                    key: "brightness".into(),
                    value: v,
                });
            }
        }
        drop(inner);

        for m in store_msgs {
            self.store.send(m).ok();
        }
        for ev in events {
            self.events.send(ev).ok();
        }
    }

    /// Validate a command and produce the MQTT publishes it maps to.
    /// Never publishes retained; expiry is applied at the MQTT layer.
    pub fn command(
        &self,
        entity_id: &str,
        cmd: &Command,
    ) -> Result<Vec<(String, String)>, interlock::Rejection> {
        let inner = self.inner.read().unwrap();
        let Some(entity) = inner.entities.get(entity_id) else {
            return Err(interlock::Rejection {
                reason: format!("unknown entity '{entity_id}'"),
            });
        };
        interlock::check(entity, cmd)?;
        let unsupported = || interlock::Rejection {
            reason: format!(
                "entity '{entity_id}' ({:?}) does not support {cmd:?}",
                entity.kind
            ),
        };
        let Some(cfg) = &entity.command else {
            return Err(unsupported());
        };

        let publishes = match (cfg, cmd) {
            (
                CommandCfg::Switch {
                    command_topic,
                    payload_on,
                    ..
                },
                Command::TurnOn,
            ) => {
                vec![(command_topic.clone(), payload_on.clone())]
            }
            (
                CommandCfg::Switch {
                    command_topic,
                    payload_off,
                    ..
                },
                Command::TurnOff,
            ) => {
                vec![(command_topic.clone(), payload_off.clone())]
            }
            (
                CommandCfg::Light(LightCmd::Basic {
                    command_topic,
                    payload_on,
                    ..
                }),
                Command::TurnOn,
            ) => {
                vec![(command_topic.clone(), payload_on.clone())]
            }
            (
                CommandCfg::Light(LightCmd::Basic {
                    command_topic,
                    payload_off,
                    ..
                }),
                Command::TurnOff,
            ) => {
                vec![(command_topic.clone(), payload_off.clone())]
            }
            (
                CommandCfg::Light(LightCmd::Basic {
                    brightness_command_topic: Some(bt),
                    brightness_scale,
                    ..
                }),
                Command::SetBrightness { brightness },
            ) => {
                let scaled = (*brightness as u32 * brightness_scale / 255).max(1);
                vec![(bt.clone(), scaled.to_string())]
            }
            (CommandCfg::Light(LightCmd::Json { command_topic, .. }), Command::TurnOn) => {
                vec![(command_topic.clone(), r#"{"state":"ON"}"#.into())]
            }
            (CommandCfg::Light(LightCmd::Json { command_topic, .. }), Command::TurnOff) => {
                vec![(command_topic.clone(), r#"{"state":"OFF"}"#.into())]
            }
            (
                CommandCfg::Light(LightCmd::Json {
                    command_topic,
                    brightness: true,
                }),
                Command::SetBrightness { brightness },
            ) => {
                vec![(
                    command_topic.clone(),
                    format!(r#"{{"state":"ON","brightness":{brightness}}}"#),
                )]
            }
            (
                CommandCfg::Cover {
                    command_topic,
                    payload_open,
                    ..
                },
                Command::Open,
            ) => {
                vec![(command_topic.clone(), payload_open.clone())]
            }
            (
                CommandCfg::Cover {
                    command_topic,
                    payload_close,
                    ..
                },
                Command::Close,
            ) => {
                vec![(command_topic.clone(), payload_close.clone())]
            }
            (
                CommandCfg::Cover {
                    command_topic,
                    payload_stop,
                    ..
                },
                Command::Stop,
            ) => {
                vec![(command_topic.clone(), payload_stop.clone())]
            }
            (
                CommandCfg::Cover {
                    set_position_topic: Some(pt),
                    ..
                },
                Command::SetPosition { position },
            ) => {
                vec![(pt.clone(), position.to_string())]
            }
            _ => return Err(unsupported()),
        };
        Ok(publishes)
    }

    /// Apply a user metadata edit. These fields survive re-announcement
    /// (see upsert) and persist through the store.
    pub fn update_meta(&self, id: &str, patch: MetaPatch) -> Option<Entity> {
        let mut inner = self.inner.write().unwrap();
        let e = inner.entities.get_mut(id)?;
        if let Some(n) = patch.name {
            let n = n.trim();
            if !n.is_empty() {
                e.name = n.to_string();
            }
        }
        if let Some(s) = patch.subsystem {
            let s = s.trim();
            e.subsystem = (!s.is_empty()).then(|| s.to_string());
        }
        if let Some(c) = patch.criticality {
            e.criticality = c;
        }
        let entity = e.clone();
        drop(inner);
        self.store
            .send(StoreMsg::UpsertEntity(Box::new(entity.clone())))
            .ok();
        self.events
            .send(Event::EntityUpserted {
                entity: Box::new(entity.clone()),
            })
            .ok();
        Some(entity)
    }

    pub fn entities_snapshot(&self) -> Vec<Entity> {
        let inner = self.inner.read().unwrap();
        let mut v: Vec<Entity> = inner.entities.values().cloned().collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    pub fn devices_snapshot(&self) -> Vec<Device> {
        let inner = self.inner.read().unwrap();
        let mut v: Vec<Device> = inner.devices.values().cloned().collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    pub fn quarantine_snapshot(&self) -> Vec<QuarantineItem> {
        let inner = self.inner.read().unwrap();
        let mut v: Vec<QuarantineItem> = inner.quarantine.values().cloned().collect();
        v.sort_by(|a, b| a.topic.cmp(&b.topic));
        v
    }

    pub fn load_quarantine(&self, items: Vec<QuarantineItem>) {
        let mut inner = self.inner.write().unwrap();
        for i in items {
            inner.quarantine.insert(i.topic.clone(), i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn registry() -> (Registry, mpsc::Receiver<StoreMsg>) {
        let (tx, rx) = mpsc::channel();
        (Registry::new(tx), rx)
    }

    const SWITCH_CFG: &[u8] =
        br#"{"uniq_id":"n1-relay","name":"Relay","stat_t":"n1/relay/state","cmd_t":"n1/relay/cmd","avty_t":"n1/status","dev":{"ids":["n1"],"name":"Node 1"}}"#;

    #[test]
    fn discovery_state_command_roundtrip() {
        let (r, _rx) = registry();
        r.handle_publish("homeassistant/switch/n1/relay/config", SWITCH_CFG, true);

        // State flows through the topic index.
        r.handle_publish("n1/relay/state", b"ON", false);
        let snap = r.entities_snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].state.as_ref().unwrap().value, Value::Bool(true));

        // Availability.
        r.handle_publish("n1/status", b"offline", false);
        assert_eq!(r.entities_snapshot()[0].available, Some(false));

        // Command maps to the declared topic/payload.
        let pubs = r.command("n1-relay", &Command::TurnOff).unwrap();
        assert_eq!(pubs, vec![("n1/relay/cmd".to_string(), "OFF".to_string())]);

        // Wrong-kind command is rejected with a reason.
        assert!(r.command("n1-relay", &Command::Open).is_err());
        assert!(r.command("ghost", &Command::TurnOn).is_err());
    }

    #[test]
    fn removal_via_empty_config_payload() {
        let (r, _rx) = registry();
        r.handle_publish("homeassistant/switch/n1/relay/config", SWITCH_CFG, true);
        assert_eq!(r.entities_snapshot().len(), 1);
        r.handle_publish("homeassistant/switch/n1/relay/config", b"", true);
        assert_eq!(r.entities_snapshot().len(), 0);
        // State on the old topic no longer routes anywhere (no panic, no ghost).
        r.handle_publish("n1/relay/state", b"ON", false);
    }

    #[test]
    fn reannounce_preserves_runtime_state() {
        let (r, _rx) = registry();
        r.handle_publish("homeassistant/switch/n1/relay/config", SWITCH_CFG, true);
        r.handle_publish("n1/relay/state", b"ON", false);
        // Device reboots and re-announces (typical ESPHome behavior).
        r.handle_publish("homeassistant/switch/n1/relay/config", SWITCH_CFG, false);
        let snap = r.entities_snapshot();
        assert_eq!(snap[0].state.as_ref().unwrap().value, Value::Bool(true));
    }

    #[test]
    fn history_points_reach_store() {
        let (r, rx) = registry();
        r.handle_publish(
            "homeassistant/sensor/t/config",
            r#"{"uniq_id":"temp1","stat_t":"n1/temp","unit_of_meas":"°C","dev":{"ids":["n1"]}}"#
                .as_bytes(),
            true,
        );
        r.handle_publish("n1/temp", b"21.5", false);
        let msgs: Vec<StoreMsg> = rx.try_iter().collect();
        assert!(msgs.iter().any(|m| matches!(m, StoreMsg::Point { entity_id, value, .. } if entity_id == "temp1" && *value == 21.5)));
    }
}
