use axm_web::events::{EventBus, GameEvent};
use std::time::Duration;

use axm_web::server::{ServerConfig, ServerError, WebServer};

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

#[tokio::test]
async fn web_server_serves_health_endpoint() {
    let server = WebServer::new(ServerConfig::for_tests()).expect("create server");
    let handle = server.start().await.expect("start server");
    let address = handle.address();

    let raw_response = fetch_health_response(address).await;

    assert!(
        raw_response.starts_with("HTTP/1.1 200"),
        "unexpected status line: {raw_response}"
    );

    let body = raw_response
        .split("\r\n\r\n")
        .nth(1)
        .expect("response body");

    let parsed: serde_json::Value = serde_json::from_str(body.trim()).expect("parse health JSON");

    assert_eq!(parsed["status"], "ok");

    tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
        .await
        .expect("shutdown timed out")
        .expect("shutdown failed");
}

#[tokio::test]
async fn web_server_reports_bind_error_when_port_in_use() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let port = listener.local_addr().expect("listener address").port();
    let static_dir = unique_static_dir("port_in_use");
    let server =
        WebServer::new(ServerConfig::new("127.0.0.1", port, static_dir)).expect("construct server");

    let err = server
        .start()
        .await
        .expect_err("expected bind error when port is in use");

    match err {
        ServerError::BindError(_) => {}
        other => panic!("expected bind error, got {:?}", other),
    }
}

async fn fetch_health_response(address: std::net::SocketAddr) -> String {
    tokio::task::spawn_blocking(move || {
        use std::io::{Read, Write};

        let mut stream = std::net::TcpStream::connect(address).expect("connect to web server");
        stream
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .expect("write request");
        let _ = stream.shutdown(std::net::Shutdown::Write);

        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).expect("read response");

        String::from_utf8(buf).expect("response utf8")
    })
    .await
    .expect("blocking task panicked")
}

fn unique_static_dir(label: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("axm_web_static_{label}_{}", uuid::Uuid::new_v4()));
    dir
}
