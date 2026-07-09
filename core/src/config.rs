//! Daemon configuration. Every field defaults so a bare `vanifold-core`
//! starts against a localhost broker; a TOML file overrides.

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub mqtt: MqttConfig,
    pub api: ApiConfig,
    pub store: StoreConfig,
    /// Entities with no update for this long degrade to quality: stale.
    pub stale_after_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub client_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiConfig {
    pub listen: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StoreConfig {
    pub db_path: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mqtt: MqttConfig::default(),
            api: ApiConfig::default(),
            store: StoreConfig::default(),
            stale_after_secs: 900,
        }
    }
}

impl Default for MqttConfig {
    fn default() -> Self {
        MqttConfig {
            host: "localhost".into(),
            port: 1883,
            username: None,
            password: None,
            client_id: "vanifold-core".into(),
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig { listen: "0.0.0.0:8480".into() }
    }
}

impl Default for StoreConfig {
    fn default() -> Self {
        StoreConfig { db_path: "vanifold.db".into() }
    }
}

impl Config {
    /// Path precedence: CLI arg, then VANIFOLD_CONFIG, then ./vanifold.toml
    /// if present, then pure defaults.
    pub fn load() -> Result<Config, String> {
        let path = std::env::args()
            .nth(1)
            .or_else(|| std::env::var("VANIFOLD_CONFIG").ok())
            .map(PathBuf::from)
            .or_else(|| {
                let p = PathBuf::from("vanifold.toml");
                p.exists().then_some(p)
            });
        match path {
            None => Ok(Config::default()),
            Some(p) => {
                let text = std::fs::read_to_string(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
                toml::from_str(&text).map_err(|e| format!("parse {}: {e}", p.display()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_partial_toml_over_defaults() {
        let cfg: Config = toml::from_str("[mqtt]\nhost = \"10.0.0.2\"\n").unwrap();
        assert_eq!(cfg.mqtt.host, "10.0.0.2");
        assert_eq!(cfg.mqtt.port, 1883);
        assert_eq!(cfg.api.listen, "0.0.0.0:8480");
    }

    #[test]
    fn rejects_unknown_fields() {
        // Typos in config should fail loudly, not silently default.
        assert!(toml::from_str::<Config>("[mqtt]\nhosst = \"x\"\n").is_err());
    }
}
