use crate::events::{EventBus, GameEvent, PlayerInfo};
use axm_engine::engine::Engine;
use axm_engine::logger::Street;
use axm_engine::player::{PlayerAction, Position};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;

pub type SessionId = String;

const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug)]
pub struct SessionManager {
    sessions: RwLock<HashMap<SessionId, Arc<GameSession>>>,
    event_bus: Arc<EventBus>,
    session_ttl: Duration,
}

impl SessionManager {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            event_bus,
            session_ttl: DEFAULT_SESSION_TTL,
        }
    }

    pub fn with_ttl(event_bus: Arc<EventBus>, ttl: Duration) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            event_bus,
            session_ttl: ttl,
        }
    }

    pub fn create_session(&self, config: GameConfig) -> Result<SessionId, SessionError> {
        let id = Uuid::new_v4().to_string();
        let session = Arc::new(GameSession::new(id.clone(), config));
        {
            let mut guard = self
                .sessions
                .write()
                .map_err(|_| SessionError::StoragePoisoned)?;
            guard.insert(id.clone(), Arc::clone(&session));
        }

        let players = session.snapshot_players();
        self.event_bus.broadcast(
            &id,
            GameEvent::GameStarted {
                session_id: id.clone(),
                players,
            },
        );

        Ok(id)
    }

    pub fn get_session(&self, id: &SessionId) -> Option<Arc<GameSession>> {
        self.sessions
            .read()
            .ok()
            .and_then(|guard| guard.get(id).cloned())
    }

    pub fn process_action(
        &self,
        session_id: &SessionId,
        action: PlayerAction,
    ) -> Result<GameEvent, SessionError> {
        let session = self
            .get_session(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.clone()))?;

        session.touch();
        let event = GameEvent::PlayerAction {
            session_id: session_id.clone(),
            player_id: 0,
            action: action.clone(),
        };
        self.event_bus.broadcast(session_id, event.clone());
        Ok(event)
    }

    pub fn cleanup_expired_sessions(&self) {
        let mut guard = match self.sessions.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.retain(|_, session| !session.is_expired(self.session_ttl));
    }

    pub fn active_sessions(&self) -> Vec<SessionId> {
        match self.sessions.read() {
            Ok(guard) => guard.keys().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct GameSession {
    id: SessionId,
    engine: Mutex<Engine>,
    config: GameConfig,
    state: Mutex<GameSessionState>,
    created_at: Instant,
    last_active: Mutex<Instant>,
}

impl GameSession {
    fn new(id: SessionId, config: GameConfig) -> Self {
        let engine = Engine::new(config.seed, config.level);
        let now = Instant::now();
        Self {
            id,
            engine: Mutex::new(engine),
            config,
            state: Mutex::new(GameSessionState::WaitingForPlayers),
            created_at: now,
            last_active: Mutex::new(now),
        }
    }

    fn snapshot_players(&self) -> Vec<PlayerInfo> {
        let engine = self.engine.lock().expect("engine lock poisoned");
        engine
            .players()
            .iter()
            .enumerate()
            .map(|(idx, player)| PlayerInfo {
                id: idx,
                stack: player.stack(),
                position: format_position(player.position()),
                is_human: idx == 0,
            })
            .collect()
    }

    fn touch(&self) {
        if let Ok(mut guard) = self.last_active.lock() {
            *guard = Instant::now();
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        match self.last_active.lock() {
            Ok(last) => last.elapsed() >= ttl,
            Err(_) => false,
        }
    }
}

fn format_position(position: Position) -> String {
    match position {
        Position::Button => "button".to_string(),
        Position::BigBlind => "big_blind".to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub seed: Option<u64>,
    pub level: u8,
    pub opponent_type: OpponentType,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            seed: None,
            level: 1,
            opponent_type: OpponentType::AI("baseline".into()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum OpponentType {
    Human,
    AI(String),
}

#[derive(Debug, Clone)]
pub enum GameSessionState {
    WaitingForPlayers,
    InProgress,
    HandInProgress {
        hand_id: String,
        current_player: usize,
        street: Street,
    },
    Completed {
        winner: Option<usize>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(SessionId),
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    #[error("Game engine error: {0}")]
    EngineError(String),
    #[error("Session expired: {0}")]
    Expired(SessionId),
    #[error("Session storage poisoned")]
    StoragePoisoned,
}
