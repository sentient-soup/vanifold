//! MQTT v5 connection: subscribes `#`, routes publishes to the registry,
//! publishes commands with replay-safety properties
//! (docs/mqtt-conventions.md: retain never set, 10s message expiry).

use crate::config::MqttConfig;
use crate::registry::Registry;
use rumqttc::v5::mqttbytes::v5::{LastWill, Packet, PublishProperties};
use rumqttc::v5::mqttbytes::QoS;
use rumqttc::v5::{AsyncClient, Event, MqttOptions};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub const STATUS_TOPIC: &str = "vanifold/core/status";
const COMMAND_EXPIRY_SECS: u32 = 10;

#[derive(Debug)]
pub struct CommandPublish {
    pub topic: String,
    pub payload: String,
}

pub async fn run(cfg: MqttConfig, registry: Arc<Registry>, mut commands: mpsc::Receiver<CommandPublish>) {
    let mut options = MqttOptions::new(cfg.client_id.clone(), cfg.host.clone(), cfg.port);
    options.set_keep_alive(Duration::from_secs(30));
    if let (Some(u), Some(p)) = (&cfg.username, &cfg.password) {
        options.set_credentials(u.clone(), p.clone());
    }
    options.set_last_will(LastWill::new(STATUS_TOPIC, "offline", QoS::AtLeastOnce, true, None));

    let (client, mut eventloop) = AsyncClient::new(options, 256);

    let publisher = client.clone();
    tokio::spawn(async move {
        while let Some(cmd) = commands.recv().await {
            let props = PublishProperties {
                message_expiry_interval: Some(COMMAND_EXPIRY_SECS),
                ..Default::default()
            };
            tracing::info!(topic = %cmd.topic, payload = %cmd.payload, "publishing command");
            // Retain is hard-false for commands; see "Command replay safety".
            if let Err(e) = publisher
                .publish_with_properties(cmd.topic, QoS::AtLeastOnce, false, cmd.payload, props)
                .await
            {
                tracing::error!(%e, "command publish failed");
            }
        }
    });

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                tracing::info!(host = %cfg.host, port = cfg.port, "mqtt connected");
                // Subscribe to everything: discovery configs route to the
                // discovery module, the rest matches registered topics.
                if let Err(e) = client.subscribe("#", QoS::AtLeastOnce).await {
                    tracing::error!(%e, "subscribe failed");
                }
                if let Err(e) = client
                    .publish(STATUS_TOPIC, QoS::AtLeastOnce, true, "online")
                    .await
                {
                    tracing::error!(%e, "status publish failed");
                }
            }
            Ok(Event::Incoming(Packet::Publish(p))) => {
                match std::str::from_utf8(&p.topic) {
                    Ok(topic) => registry.handle_publish(topic, &p.payload, p.retain),
                    Err(_) => tracing::debug!("ignoring publish with non-utf8 topic"),
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(%e, "mqtt connection error, retrying in 3s");
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }
}
