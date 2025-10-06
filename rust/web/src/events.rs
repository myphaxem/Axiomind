use crate::session::SessionId;
use axm_engine::player::PlayerAction;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

pub type EventSender = mpsc::UnboundedSender<GameEvent>;
pub type EventReceiver = mpsc::UnboundedReceiver<GameEvent>;

#[derive(Debug, Clone, Default)]
pub struct EventBus {
    inner: Arc<EventBusInner>,
}

#[derive(Debug, Default)]
struct EventBusInner {
    subscribers: RwLock<HashMap<SessionId, Vec<(usize, EventSender)>>>,
    next_id: AtomicUsize,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self, session_id: SessionId) -> (usize, EventReceiver) {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = self.inner.next_id.fetch_add(1, Ordering::AcqRel);
        let mut guard = self
            .inner
            .subscribers
            .write()
            .expect("subscriber lock poisoned");
        guard.entry(session_id).or_default().push((id, tx));
        (id, rx)
    }

    pub fn broadcast(&self, session_id: &SessionId, event: GameEvent) {
        let subscribers = {
            let guard = self
                .inner
                .subscribers
                .read()
                .expect("subscriber lock poisoned");
            guard.get(session_id).cloned()
        };

        if let Some(list) = subscribers {
            let mut failed = Vec::new();
            for (id, sender) in list {
                if sender.send(event.clone()).is_err() {
                    failed.push(id);
                }
            }
            if !failed.is_empty() {
                self.remove_subscribers(session_id, &failed);
            }
        }
    }

    pub fn unsubscribe(&self, session_id: &SessionId, subscriber_id: usize) {
        self.remove_subscribers(session_id, &[subscriber_id]);
    }

    pub fn subscriber_count(&self) -> usize {
        let guard = self
            .inner
            .subscribers
            .read()
            .expect("subscriber lock poisoned");
        guard.values().map(|list| list.len()).sum()
    }

    fn remove_subscribers(&self, session_id: &SessionId, ids: &[usize]) {
        let mut guard = self
            .inner
            .subscribers
            .write()
            .expect("subscriber lock poisoned");
        if let Some(list) = guard.get_mut(session_id) {
            list.retain(|(id, _)| !ids.contains(id));
            if list.is_empty() {
                guard.remove(session_id);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
    GameStarted {
        session_id: SessionId,
        players: Vec<PlayerInfo>,
    },
    PlayerAction {
        session_id: SessionId,
        player_id: usize,
        action: PlayerAction,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerInfo {
    pub id: usize,
    pub stack: u32,
    pub position: String,
    pub is_human: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct HandResult {
    pub winner_ids: Vec<usize>,
    pub pot: u32,
}
