use crate::events::{EventBus, GameEvent, PlayerInfo};
use axm_engine::cards::Card;
use axm_engine::engine::Engine;
use axm_engine::logger::Street;
use axm_engine::player::{PlayerAction, Position as EnginePosition};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;

pub type SessionId = String;

const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatPosition {
    Button,
    BigBlind,
}

impl From<EnginePosition> for SeatPosition {
    fn from(position: EnginePosition) -> Self {
        match position {
            EnginePosition::Button => SeatPosition::Button,
            EnginePosition::BigBlind => SeatPosition::BigBlind,
        }
    }
}

impl From<SeatPosition> for EnginePosition {
    fn from(position: SeatPosition) -> Self {
        match position {
            SeatPosition::Button => EnginePosition::Button,
            SeatPosition::BigBlind => EnginePosition::BigBlind,
        }
    }
}

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
                position: SeatPosition::from(player.position()),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpponentType {
    Human,
    AI(String),
}

impl OpponentType {
    fn as_str(&self) -> Cow<'_, str> {
        match self {
            OpponentType::Human => Cow::Borrowed("human"),
            OpponentType::AI(name) => {
                let mut value = String::with_capacity(3 + name.len());
                value.push_str("ai:");
                value.push_str(name);
                Cow::Owned(value)
            }
        }
    }
}

impl Serialize for OpponentType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.as_str())
    }
}

impl<'de> Deserialize<'de> for OpponentType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        if raw.eq_ignore_ascii_case("human") {
            return Ok(OpponentType::Human);
        }

        if let Some(rest) = raw.strip_prefix("ai:") {
            if rest.is_empty() {
                return Ok(OpponentType::AI("baseline".into()));
            }
            return Ok(OpponentType::AI(rest.to_string()));
        }

        Err(serde::de::Error::custom(format!(
            "invalid opponent type: {raw}"
        )))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvailableAction {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayerStateResponse {
    pub id: usize,
    pub stack: u32,
    pub position: SeatPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hole_cards: Option<Vec<Card>>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_action: Option<PlayerAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameStateResponse {
    pub session_id: SessionId,
    pub players: Vec<PlayerStateResponse>,
    pub board: Vec<Card>,
    pub pot: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_player: Option<usize>,
    pub available_actions: Vec<AvailableAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hand_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street: Option<Street>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
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
