//! HTTP/WebSocket API: the only egress in v0.1 (docs/mqtt-conventions.md).
//! REST for snapshots and history, WebSocket for the live stream + commands.

use crate::model::{now_ms, Command, Entity};
use crate::mqtt::CommandPublish;
use crate::registry::Registry;
use crate::store;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct App {
    pub registry: Arc<Registry>,
    pub commands: mpsc::Sender<CommandPublish>,
    pub db_path: PathBuf,
    pub stale_after_ms: u64,
}

/// Entities serialize with their computed quality attached; quality is derived
/// (age + availability), never stored.
fn entities_json(app: &App) -> Vec<serde_json::Value> {
    let now = now_ms();
    app.registry
        .entities_snapshot()
        .iter()
        .map(|e: &Entity| {
            let mut v = serde_json::to_value(e).unwrap_or_default();
            if let Some(obj) = v.as_object_mut() {
                obj.insert(
                    "quality".into(),
                    serde_json::to_value(e.quality(now, app.stale_after_ms)).unwrap_or_default(),
                );
            }
            v
        })
        .collect()
}

fn snapshot_json(app: &App) -> serde_json::Value {
    serde_json::json!({
        "type": "snapshot",
        "entities": entities_json(app),
        "devices": app.registry.devices_snapshot(),
    })
}

pub fn router(app: App) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/entities", get(entities))
        .route("/api/devices", get(devices))
        .route("/api/quarantine", get(quarantine))
        .route("/api/history/{entity_id}", get(history))
        .route("/api/ws", get(ws_upgrade))
        .with_state(app)
}

async fn index() -> &'static str {
    // ponytail: placeholder until the SvelteKit build is embedded here.
    "vanifold-core is running. API at /api, live stream at /api/ws."
}

async fn entities(State(app): State<App>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "entities": entities_json(&app) }))
}

async fn devices(State(app): State<App>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "devices": app.registry.devices_snapshot() }))
}

async fn quarantine(State(app): State<App>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "quarantine": app.registry.quarantine_snapshot() }))
}

#[derive(Deserialize)]
struct HistoryRange {
    from: Option<u64>,
    to: Option<u64>,
}

async fn history(
    State(app): State<App>,
    Path(entity_id): Path<String>,
    Query(range): Query<HistoryRange>,
) -> Response {
    let now = crate::model::now_ms();
    let from = range.from.unwrap_or(now.saturating_sub(24 * 3600 * 1000));
    let to = range.to.unwrap_or(now);
    let path = app.db_path.clone();
    let result = tokio::task::spawn_blocking(move || store::history(&path, &entity_id, from, to)).await;
    match result {
        Ok(Ok(points)) => Json(serde_json::json!({ "points": points })).into_response(),
        Ok(Err(e)) => {
            tracing::error!(%e, "history query failed");
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "history query failed").into_response()
        }
        Err(e) => {
            tracing::error!(%e, "history task failed");
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "history task failed").into_response()
        }
    }
}

/// Client -> server over the socket.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Command {
        /// Correlation id echoed back in the result.
        #[serde(default)]
        id: Option<String>,
        entity_id: String,
        #[serde(flatten)]
        command: Command,
    },
}

async fn ws_upgrade(ws: WebSocketUpgrade, State(app): State<App>) -> Response {
    ws.on_upgrade(|socket| ws_session(socket, app))
}

async fn ws_session(mut socket: WebSocket, app: App) {
    // Snapshot first so the client renders immediately, then the live stream.
    if socket.send(Message::text(snapshot_json(&app).to_string())).await.is_err() {
        return;
    }
    let mut events = app.registry.subscribe();

    loop {
        tokio::select! {
            event = events.recv() => {
                match event {
                    Ok(ev) => {
                        let text = serde_json::to_string(&ev).unwrap_or_default();
                        if socket.send(Message::text(text)).await.is_err() {
                            return;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // Slow client: resync with a fresh snapshot rather than
                        // silently missing events.
                        tracing::warn!(missed = n, "ws client lagged, resyncing");
                        if socket.send(Message::text(snapshot_json(&app).to_string())).await.is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
            incoming = socket.recv() => {
                let Some(Ok(msg)) = incoming else { return };
                if let Message::Text(text) = msg {
                    let reply = handle_client_msg(&app, &text).await;
                    if socket.send(Message::text(reply.to_string())).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}

async fn handle_client_msg(app: &App, text: &str) -> serde_json::Value {
    let parsed: Result<ClientMsg, _> = serde_json::from_str(text);
    match parsed {
        Ok(ClientMsg::Command { id, entity_id, command }) => {
            match app.registry.command(&entity_id, &command) {
                Ok(publishes) => {
                    for (topic, payload) in publishes {
                        if app.commands.send(CommandPublish { topic, payload }).await.is_err() {
                            return serde_json::json!({
                                "type": "result", "id": id, "ok": false,
                                "reason": "command channel closed"
                            });
                        }
                    }
                    // Published, not confirmed: the state echo arrives on the
                    // stream (confirmed-not-optimistic, docs/mqtt-conventions.md).
                    serde_json::json!({ "type": "result", "id": id, "ok": true })
                }
                Err(rejection) => serde_json::json!({
                    "type": "result", "id": id, "ok": false, "reason": rejection.reason
                }),
            }
        }
        Err(e) => serde_json::json!({ "type": "result", "ok": false, "reason": format!("bad message: {e}") }),
    }
}
