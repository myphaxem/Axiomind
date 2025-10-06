use crate::events::EventBus;
use crate::session::{SessionError, SessionManager};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    host: String,
    port: u16,
    static_dir: PathBuf,
}

impl ServerConfig {
    pub fn new(host: impl Into<String>, port: u16, static_dir: impl Into<PathBuf>) -> Self {
        Self {
            host: host.into(),
            port,
            static_dir: static_dir.into(),
        }
    }

    pub fn for_tests() -> Self {
        let dir = std::env::temp_dir().join("axm_web_static");
        Self::new("127.0.0.1", 0, dir)
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn static_dir(&self) -> &Path {
        &self.static_dir
    }
}

#[derive(Debug, Clone)]
pub struct AppContext {
    config: ServerConfig,
    event_bus: Arc<EventBus>,
    sessions: Arc<SessionManager>,
}

impl AppContext {
    pub fn new(config: ServerConfig) -> Result<Self, ServerError> {
        if !config.static_dir().exists() {
            fs::create_dir_all(config.static_dir())
                .map_err(|err| ServerError::ConfigError(err.to_string()))?;
        }

        let event_bus = Arc::new(EventBus::new());
        let sessions = Arc::new(SessionManager::new(Arc::clone(&event_bus)));
        Ok(Self::new_with_dependencies(config, event_bus, sessions))
    }

    pub fn new_with_dependencies(
        config: ServerConfig,
        event_bus: Arc<EventBus>,
        sessions: Arc<SessionManager>,
    ) -> Self {
        Self {
            config,
            event_bus,
            sessions,
        }
    }

    pub fn new_for_tests() -> Self {
        Self::new(ServerConfig::for_tests()).expect("test context")
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    pub fn sessions(&self) -> Arc<SessionManager> {
        Arc::clone(&self.sessions)
    }
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Failed to bind to address: {0}")]
    BindError(#[from] std::io::Error),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Session error: {0}")]
    SessionError(#[from] SessionError),
}
