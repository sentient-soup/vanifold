//! SQLite persistence: registry snapshots, quarantine, and tiered history
//! (docs/DESIGN.md: raw ~48h -> 1-min ~30d -> 1-hour forever). Runs on a
//! dedicated thread; writes are batched to be SD-card-friendly.

use crate::model::{Device, Entity, now_ms};
use crate::registry::QuarantineItem;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

pub const RAW_RETENTION_MS: u64 = 48 * 3600 * 1000;
pub const MINUTE_RETENTION_MS: u64 = 30 * 24 * 3600 * 1000;
const FLUSH_INTERVAL: Duration = Duration::from_secs(5);
const DOWNSAMPLE_INTERVAL_MS: u64 = 3600 * 1000;

#[derive(Debug)]
pub enum StoreMsg {
    UpsertDevice(Box<Device>),
    UpsertEntity(Box<Entity>),
    RemoveEntity(String),
    Quarantine(QuarantineItem),
    RemoveQuarantine(String),
    Point {
        entity_id: String,
        ts: u64,
        value: f64,
    },
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS devices(id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS entities(id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS quarantine(topic TEXT PRIMARY KEY, payload TEXT NOT NULL, reason TEXT NOT NULL, ts INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS history_raw(entity_id TEXT NOT NULL, ts INTEGER NOT NULL, value REAL NOT NULL);
CREATE INDEX IF NOT EXISTS idx_raw ON history_raw(entity_id, ts);
CREATE TABLE IF NOT EXISTS history_1m(entity_id TEXT NOT NULL, ts INTEGER NOT NULL, vmin REAL, vmax REAL, vavg REAL, n INTEGER, PRIMARY KEY(entity_id, ts));
CREATE TABLE IF NOT EXISTS history_1h(entity_id TEXT NOT NULL, ts INTEGER NOT NULL, vmin REAL, vmax REAL, vavg REAL, n INTEGER, PRIMARY KEY(entity_id, ts));
";

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

pub struct Loaded {
    pub devices: Vec<Device>,
    pub entities: Vec<Entity>,
    pub quarantine: Vec<QuarantineItem>,
}

pub fn load(conn: &Connection) -> rusqlite::Result<Loaded> {
    let mut devices = Vec::new();
    let mut stmt = conn.prepare("SELECT json FROM devices")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let json: String = row.get(0)?;
        if let Ok(d) = serde_json::from_str(&json) {
            devices.push(d);
        }
    }
    let mut entities = Vec::new();
    let mut stmt = conn.prepare("SELECT json FROM entities")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let json: String = row.get(0)?;
        if let Ok(e) = serde_json::from_str(&json) {
            entities.push(e);
        }
    }
    let mut quarantine = Vec::new();
    let mut stmt = conn.prepare("SELECT topic, payload, reason, ts FROM quarantine")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        quarantine.push(QuarantineItem {
            topic: row.get(0)?,
            payload: row.get(1)?,
            reason: row.get(2)?,
            ts: row.get::<_, i64>(3)? as u64,
        });
    }
    Ok(Loaded {
        devices,
        entities,
        quarantine,
    })
}

/// Spawn the writer thread. All mutations flow through the returned sender.
pub fn spawn(path: PathBuf) -> (Sender<StoreMsg>, std::thread::JoinHandle<()>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let handle = std::thread::Builder::new()
        .name("vanifold-store".into())
        .spawn(move || {
            let conn = match open(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!(?path, %e, "store: cannot open database, persistence disabled");
                    // Drain forever so senders never error out.
                    for _ in rx {}
                    return;
                }
            };
            run(conn, rx);
        })
        .expect("spawn store thread");
    (tx, handle)
}

fn run(mut conn: Connection, rx: Receiver<StoreMsg>) {
    let mut points: Vec<(String, u64, f64)> = Vec::new();
    let mut last_downsample: u64 = 0;
    loop {
        match rx.recv_timeout(FLUSH_INTERVAL) {
            Ok(msg) => apply(&conn, msg, &mut points),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                flush(&mut conn, &mut points);
                return;
            }
        }
        // Batched, SD-friendly: one transaction per interval, not per message.
        if !points.is_empty() {
            flush(&mut conn, &mut points);
        }
        let now = now_ms();
        if now.saturating_sub(last_downsample) > DOWNSAMPLE_INTERVAL_MS {
            last_downsample = now;
            if let Err(e) = downsample(&conn, now) {
                tracing::error!(%e, "store: downsample failed");
            }
        }
    }
}

fn apply(conn: &Connection, msg: StoreMsg, points: &mut Vec<(String, u64, f64)>) {
    let result = match msg {
        StoreMsg::UpsertDevice(d) => conn
            .execute(
                "INSERT OR REPLACE INTO devices(id, json) VALUES(?1, ?2)",
                (&d.id, serde_json::to_string(&d).unwrap_or_default()),
            )
            .map(|_| ()),
        StoreMsg::UpsertEntity(e) => conn
            .execute(
                "INSERT OR REPLACE INTO entities(id, json) VALUES(?1, ?2)",
                (&e.id, serde_json::to_string(&e).unwrap_or_default()),
            )
            .map(|_| ()),
        StoreMsg::RemoveEntity(id) => conn.execute("DELETE FROM entities WHERE id = ?1", [&id]).map(|_| ()),
        StoreMsg::Quarantine(q) => conn
            .execute(
                "INSERT OR REPLACE INTO quarantine(topic, payload, reason, ts) VALUES(?1, ?2, ?3, ?4)",
                (&q.topic, &q.payload, &q.reason, q.ts as i64),
            )
            .map(|_| ()),
        StoreMsg::RemoveQuarantine(topic) => {
            conn.execute("DELETE FROM quarantine WHERE topic = ?1", [&topic]).map(|_| ())
        }
        StoreMsg::Point { entity_id, ts, value } => {
            points.push((entity_id, ts, value));
            Ok(())
        }
    };
    if let Err(e) = result {
        tracing::error!(%e, "store: write failed");
    }
}

fn flush(conn: &mut Connection, points: &mut Vec<(String, u64, f64)>) {
    if points.is_empty() {
        return;
    }
    let result = (|| -> rusqlite::Result<()> {
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO history_raw(entity_id, ts, value) VALUES(?1, ?2, ?3)",
            )?;
            for (id, ts, v) in points.iter() {
                stmt.execute((id, *ts as i64, *v))?;
            }
        }
        tx.commit()
    })();
    match result {
        Ok(()) => points.clear(),
        Err(e) => {
            tracing::error!(%e, n = points.len(), "store: history flush failed, dropping batch");
            points.clear(); // ponytail: drop rather than grow unbounded; telemetry, not ledger
        }
    }
}

/// Move raw rows older than the retention window into 1-minute buckets, and
/// 1-minute rows past theirs into 1-hour buckets. Cutoffs are bucket-aligned
/// so a bucket is aggregated exactly once.
pub fn downsample(conn: &Connection, now: u64) -> rusqlite::Result<()> {
    let raw_cutoff = ((now.saturating_sub(RAW_RETENTION_MS)) / 60_000 * 60_000) as i64;
    conn.execute_batch(&format!(
        "BEGIN;
         INSERT OR REPLACE INTO history_1m(entity_id, ts, vmin, vmax, vavg, n)
           SELECT entity_id, (ts/60000)*60000, MIN(value), MAX(value), AVG(value), COUNT(*)
           FROM history_raw WHERE ts < {raw_cutoff} GROUP BY entity_id, ts/60000;
         DELETE FROM history_raw WHERE ts < {raw_cutoff};
         COMMIT;"
    ))?;
    let minute_cutoff = ((now.saturating_sub(MINUTE_RETENTION_MS)) / 3_600_000 * 3_600_000) as i64;
    conn.execute_batch(&format!(
        "BEGIN;
         INSERT OR REPLACE INTO history_1h(entity_id, ts, vmin, vmax, vavg, n)
           SELECT entity_id, (ts/3600000)*3600000, MIN(vmin), MAX(vmax), SUM(vavg*n)/SUM(n), SUM(n)
           FROM history_1m WHERE ts < {minute_cutoff} GROUP BY entity_id, ts/3600000;
         DELETE FROM history_1m WHERE ts < {minute_cutoff};
         COMMIT;"
    ))?;
    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct HistoryPoint {
    pub ts: u64,
    pub vmin: f64,
    pub vmax: f64,
    pub vavg: f64,
}

/// Read history across tiers for one entity. Picks per tier by time range;
/// raw points come back as degenerate buckets (min = max = avg).
pub fn history(
    path: &Path,
    entity_id: &str,
    from: u64,
    to: u64,
) -> rusqlite::Result<Vec<HistoryPoint>> {
    let conn = Connection::open(path)?;
    let mut out = Vec::new();
    for (table, is_raw) in [
        ("history_1h", false),
        ("history_1m", false),
        ("history_raw", true),
    ] {
        let sql = if is_raw {
            format!(
                "SELECT ts, value, value, value FROM {table} WHERE entity_id = ?1 AND ts >= ?2 AND ts <= ?3 ORDER BY ts"
            )
        } else {
            format!(
                "SELECT ts, vmin, vmax, vavg FROM {table} WHERE entity_id = ?1 AND ts >= ?2 AND ts <= ?3 ORDER BY ts"
            )
        };
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query((entity_id, from as i64, to as i64))?;
        while let Some(row) = rows.next()? {
            out.push(HistoryPoint {
                ts: row.get::<_, i64>(0)? as u64,
                vmin: row.get(1)?,
                vmax: row.get(2)?,
                vavg: row.get(3)?,
            });
        }
    }
    out.sort_by_key(|p| p.ts);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsample_tiers() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        let now: u64 = 100 * 24 * 3600 * 1000; // fixed fake "now"

        // Two raw points in the same old minute bucket, one recent point.
        let old = now - RAW_RETENTION_MS - 120_000;
        let bucket = old / 60_000 * 60_000;
        for (ts, v) in [(old, 10.0), (old + 1000, 20.0), (now - 1000, 5.0)] {
            conn.execute(
                "INSERT INTO history_raw(entity_id, ts, value) VALUES('e1', ?1, ?2)",
                (ts as i64, v),
            )
            .unwrap();
        }
        downsample(&conn, now).unwrap();

        // Old points rolled into one 1m bucket; recent point untouched.
        let (ts, vmin, vmax, vavg, n): (i64, f64, f64, f64, i64) = conn
            .query_row("SELECT ts, vmin, vmax, vavg, n FROM history_1m", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .unwrap();
        assert_eq!(ts as u64, bucket);
        assert_eq!((vmin, vmax, vavg, n), (10.0, 20.0, 15.0, 2));
        let raw_left: i64 = conn
            .query_row("SELECT COUNT(*) FROM history_raw", [], |r| r.get(0))
            .unwrap();
        assert_eq!(raw_left, 1);

        // Age the 1m bucket past 30d: it rolls into 1h with weighted average.
        let much_later = now + MINUTE_RETENTION_MS + 3_600_000;
        downsample(&conn, much_later).unwrap();
        let (vavg_1h, n_1h): (f64, i64) = conn
            .query_row("SELECT vavg, n FROM history_1h", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((vavg_1h, n_1h), (15.0, 2));
        let m_left: i64 = conn
            .query_row("SELECT COUNT(*) FROM history_1m", [], |r| r.get(0))
            .unwrap();
        assert_eq!(m_left, 0);
    }

    #[test]
    fn registry_persistence_roundtrip() {
        let dir = std::env::temp_dir().join(format!("vanifold-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let _ = std::fs::remove_file(&path);

        let conn = open(&path).unwrap();
        let d = Device {
            id: "d1".into(),
            name: "Device".into(),
            manufacturer: None,
            model: None,
            sw_version: None,
            hw_version: None,
            identifiers: vec!["d1".into()],
        };
        conn.execute(
            "INSERT OR REPLACE INTO devices(id, json) VALUES(?1, ?2)",
            (&d.id, serde_json::to_string(&d).unwrap()),
        )
        .unwrap();
        let loaded = load(&conn).unwrap();
        assert_eq!(loaded.devices.len(), 1);
        assert_eq!(loaded.devices[0].id, "d1");
        drop(conn);
        let _ = std::fs::remove_file(&path);
    }
}
