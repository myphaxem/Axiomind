pub mod events;
pub mod handlers;
pub mod server;
pub mod session;

pub use server::{ServerHandle, WebServer};
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_provides_shared_components() {
        let ctx = AppContext::new_for_tests();

        let event_bus = ctx.event_bus();
        let sessions = ctx.sessions();

        assert_eq!(event_bus.subscriber_count(), 0);
        assert!(sessions.active_sessions().is_empty());
    }
}

pub use events::{EventBus, GameEvent, PlayerInfo};
pub use server::{AppContext, ServerConfig, ServerError};
pub use session::{GameConfig, OpponentType, SessionError, SessionId, SessionManager};
