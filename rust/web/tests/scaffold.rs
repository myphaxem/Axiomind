use axm_web::events::{EventBus, GameEvent};
use std::time::Duration;

#[tokio::test]
async fn event_bus_broadcasts_error_events() {
    let bus = EventBus::new();
    let session_id = "session".to_string();
    let (_id, mut rx) = bus.subscribe(session_id.clone());

    bus.broadcast(
        &session_id,
        GameEvent::Error {
            message: "ping".into(),
        },
    );

    let received = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .expect("channel receive timed out")
        .expect("channel unexpectedly closed");

    match received {
        GameEvent::Error { message } => assert_eq!(message, "ping"),
        other => panic!("unexpected event: {:?}", other),
    }
}
