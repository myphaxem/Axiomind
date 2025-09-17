use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub starting_stack: u32,
    pub level: u8,
    pub seed: Option<u64>,
    pub adaptive: bool,
    pub ai_version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            starting_stack: 20_000,
            level: 1,
            seed: None,
            adaptive: true,
            ai_version: "latest".into(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    Invalid(String),
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e)
    }
}
impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::Parse(e)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn load() -> Result<Config, ConfigError> {
    let mut cfg = Config::default();
    if let Ok(path) = std::env::var("AXM_CONFIG") {
        let s = fs::read_to_string(path)?;
        let f: FileConfig = toml::from_str(&s)?;
        if let Some(v) = f.starting_stack {
            cfg.starting_stack = v;
        }
        if let Some(v) = f.level {
            cfg.level = v;
        }
        if let Some(v) = f.seed {
            cfg.seed = Some(v);
        }
        if let Some(v) = f.adaptive {
            cfg.adaptive = v;
        }
        if let Some(v) = f.ai_version {
            cfg.ai_version = v;
        }
    }

    if let Ok(seed) = std::env::var("AXM_SEED") {
        if !seed.is_empty() {
            cfg.seed = Some(
                seed.parse()
                    .map_err(|_| ConfigError::Invalid("Invalid seed".into()))?,
            );
        }
    }
    if let Ok(level) = std::env::var("AXM_LEVEL") {
        if !level.is_empty() {
            cfg.level = level
                .parse()
                .map_err(|_| ConfigError::Invalid("Invalid level".into()))?;
        }
    }
    if let Ok(adap) = std::env::var("AXM_ADAPTIVE") {
        if !adap.is_empty() {
            cfg.adaptive =
                parse_bool(&adap).ok_or_else(|| ConfigError::Invalid("Invalid adaptive".into()))?;
        }
    }
    if let Ok(ver) = std::env::var("AXM_AI_VERSION") {
        if !ver.is_empty() {
            cfg.ai_version = ver;
        }
    }

    validate(&cfg)?;
    Ok(cfg)
}

#[derive(Debug, Deserialize)]
struct FileConfig {
    #[serde(default)]
    starting_stack: Option<u32>,
    #[serde(default)]
    level: Option<u8>,
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default)]
    adaptive: Option<bool>,
    #[serde(default)]
    ai_version: Option<String>,
}

fn validate(cfg: &Config) -> Result<(), ConfigError> {
    if cfg.level == 0 {
        return Err(ConfigError::Invalid(
            "Invalid configuration: level must be >=1".into(),
        ));
    }
    if cfg.starting_stack == 0 {
        return Err(ConfigError::Invalid(
            "Invalid configuration: starting_stack must be >0".into(),
        ));
    }
    Ok(())
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}
