use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub modules: Modules,
    pub adsb: AdsbConfig,
    pub ais: AisConfig,
    pub api: ApiConfig,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Modules {
    pub adsb: bool,
    pub ais: bool,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct AdsbConfig {
    pub ingest_port: u16,
    pub db: bool,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct AisConfig {
    pub nmea_host: String,
    pub db: bool,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct ApiConfig {
    pub port: u16,
    pub ws: bool,
    pub mcp: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            modules: Modules::default(),
            adsb: AdsbConfig::default(),
            ais: AisConfig::default(),
            api: ApiConfig::default(),
        }
    }
}

impl Default for Modules {
    fn default() -> Self { Self { adsb: true, ais: false } }
}

impl Default for AdsbConfig {
    fn default() -> Self { Self { ingest_port: 39004, db: true } }
}

impl Default for AisConfig {
    fn default() -> Self { Self { nmea_host: "127.0.0.1:10110".into(), db: true } }
}

impl Default for ApiConfig {
    fn default() -> Self { Self { port: 19180, ws: true, mcp: true } }
}

impl Config {
    pub fn load() -> Self {
        for p in &["skylink.toml", "/etc/skylink/skylink.toml"] {
            if Path::new(p).exists() {
                if let Ok(s) = std::fs::read_to_string(p) {
                    if let Ok(c) = toml::from_str(&s) {
                        tracing::info!("config loaded from {p}");
                        return c;
                    }
                }
            }
        }
        tracing::info!("no config file, using defaults (adsb=true, ais=false)");
        Self::default()
    }

    /// Apply CLI overrides: --adsb, --ais, --no-adsb, --no-ais
    pub fn apply_cli(&mut self) {
        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--adsb" => self.modules.adsb = true,
                "--no-adsb" => self.modules.adsb = false,
                "--ais" => self.modules.ais = true,
                "--no-ais" => self.modules.ais = false,
                _ => {}
            }
        }
        // Also check env vars for backward compat
        if let Ok(v) = std::env::var("INGEST_PORT") { if let Ok(p) = v.parse() { self.adsb.ingest_port = p; } }
        if let Ok(v) = std::env::var("API_PORT") { if let Ok(p) = v.parse() { self.api.port = p; } }
        if let Ok(v) = std::env::var("NMEA_HOST") { self.ais.nmea_host = v; }
    }
}
