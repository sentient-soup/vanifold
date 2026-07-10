mod api;
mod config;
mod discovery;
mod interlock;
mod model;
mod mqtt;
mod registry;
mod store;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).init();

    let cfg = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("config error: {e}");
            std::process::exit(1);
        }
    };
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "vanifold-core starting");

    // Store thread first; the registry persists through it.
    let (store_tx, _store_handle) = store::spawn(cfg.store.db_path.clone());
    let registry = Arc::new(registry::Registry::new(store_tx));

    // Seed from disk so entities exist before the broker reconnects.
    match store::open(&cfg.store.db_path).and_then(|conn| store::load(&conn)) {
        Ok(loaded) => {
            tracing::info!(
                devices = loaded.devices.len(),
                entities = loaded.entities.len(),
                quarantined = loaded.quarantine.len(),
                "loaded registry from store"
            );
            registry.load(loaded.devices, loaded.entities);
            registry.load_quarantine(loaded.quarantine);
        }
        Err(e) => tracing::warn!(%e, "could not load registry from store, starting empty"),
    }

    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(64);
    tokio::spawn(mqtt::run(cfg.mqtt.clone(), registry.clone(), cmd_rx));

    let app = api::App {
        registry,
        commands: cmd_tx,
        db_path: cfg.store.db_path.clone(),
        stale_after_ms: cfg.stale_after_secs * 1000,
    };
    let listener = match tokio::net::TcpListener::bind(&cfg.api.listen).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(listen = %cfg.api.listen, %e, "cannot bind API listener");
            std::process::exit(1);
        }
    };
    let ui_dir = cfg.api.ui_dir.join("index.html").exists().then(|| cfg.api.ui_dir.clone());
    match &ui_dir {
        Some(d) => tracing::info!(dir = %d.display(), "serving UI"),
        None => tracing::info!(dir = %cfg.api.ui_dir.display(), "no UI build found, API only"),
    }
    tracing::info!(listen = %cfg.api.listen, "API listening");
    if let Err(e) = axum::serve(listener, api::router(app, ui_dir)).await {
        tracing::error!(%e, "API server exited");
    }
}
