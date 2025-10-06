pub mod events;
pub mod handlers;
pub mod server;
pub mod session;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_provides_shared_components() {
        let ctx = AppContext::new_for_tests();

        let event_bus = ctx.event_bus();
        let sessions = ctx.sessions();

        assert!(event_bus.subscriber_count() >= 0);
        assert!(sessions.active_sessions().is_empty());
    }
}

pub use events::EventBus;
pub use server::AppContext;
pub use session::{SessionId, SessionManager};
