use axm_web::events::{EventBus, GameEvent};
use std::time::Duration;

use axm_web::server::{ServerConfig, ServerError, WebServer};
use std::fs;
use std::path::{Path, PathBuf};
use warp::hyper::{self, Client as HyperClient};

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

    let client = HyperClient::new();

    tokio::time::sleep(Duration::from_millis(20)).await;

    let uri: hyper::Uri = format!("http://{address}/health")
        .parse()
        .expect("parse uri");

    let response = client.get(uri).await.expect("request /health succeeded");

    assert_eq!(response.status(), hyper::StatusCode::OK);

    let body_bytes = hyper::body::to_bytes(response.into_body())
        .await
        .expect("read health body");

    let parsed: serde_json::Value = serde_json::from_slice(&body_bytes).expect("parse health JSON");

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

#[tokio::test]
async fn web_server_serves_index_html() {
    let fixture = create_static_fixture("index_html");
    let server =
        WebServer::new(ServerConfig::new("127.0.0.1", 0, &fixture)).expect("create server");
    let handle = server.start().await.expect("start server");
    let address = handle.address();
    let client = HyperClient::new();

    tokio::time::sleep(Duration::from_millis(20)).await;

    let uri: hyper::Uri = format!("http://{address}/").parse().expect("parse uri");
    let response = client.get(uri).await.expect("request index");
    let (parts, body) = response.into_parts();

    let content_type = parts
        .headers
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .expect("content-type");
    assert_eq!(content_type, "text/html; charset=utf-8");

    let cache_control = parts
        .headers
        .get(hyper::header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
        .expect("cache-control");
    assert_eq!(cache_control, "public, max-age=86400");

    let body_bytes = hyper::body::to_bytes(body).await.expect("read body");
    let body_text = String::from_utf8(body_bytes.to_vec()).expect("utf8 body");
    assert!(body_text.contains("htmx.min.js"));
    assert!(body_text.contains("app.css"));

    tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
        .await
        .expect("shutdown timed out")
        .expect("shutdown failed");

    let _ = fs::remove_dir_all(&fixture);
}

#[tokio::test]
async fn web_server_serves_static_assets() {
    let fixture = create_static_fixture("static_assets");
    let server =
        WebServer::new(ServerConfig::new("127.0.0.1", 0, &fixture)).expect("create server");
    let handle = server.start().await.expect("start server");
    let address = handle.address();
    let client = HyperClient::new();

    tokio::time::sleep(Duration::from_millis(20)).await;

    let css_uri: hyper::Uri = format!("http://{address}/static/css/app.css")
        .parse()
        .expect("parse css uri");
    let css_response = client.get(css_uri).await.expect("request css");
    let (parts, body) = css_response.into_parts();
    let content_type = parts
        .headers
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .expect("content-type");
    assert_eq!(content_type, "text/css");
    let cache_control = parts
        .headers
        .get(hyper::header::CACHE_CONTROL)
        .and_then(|v| v.to_str().ok())
        .expect("cache-control");
    assert_eq!(cache_control, "public, max-age=86400");
    let body_bytes = hyper::body::to_bytes(body).await.expect("read css body");
    let body_text = String::from_utf8(body_bytes.to_vec()).expect("utf8 body");
    assert!(body_text.contains("table-layout"));

    let js_uri: hyper::Uri = format!("http://{address}/static/js/htmx.min.js")
        .parse()
        .expect("parse js uri");
    let js_response = client.get(js_uri).await.expect("request js");
    let (parts, body) = js_response.into_parts();
    let js_content_type = parts
        .headers
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .expect("content-type");
    assert_eq!(js_content_type, "application/javascript");

    let js_body = hyper::body::to_bytes(body).await.expect("read js");
    let js_text = String::from_utf8(js_body.to_vec()).expect("utf8 js");
    assert!(js_text.contains("htmx"));

    tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
        .await
        .expect("shutdown timed out")
        .expect("shutdown failed");

    let _ = fs::remove_dir_all(&fixture);
}

#[tokio::test]
async fn web_server_returns_404_for_missing_asset() {
    let fixture = create_static_fixture("missing_asset");
    let server =
        WebServer::new(ServerConfig::new("127.0.0.1", 0, &fixture)).expect("create server");
    let handle = server.start().await.expect("start server");
    let address = handle.address();
    let client = HyperClient::new();

    tokio::time::sleep(Duration::from_millis(20)).await;

    let uri: hyper::Uri = format!("http://{address}/static/js/missing.js")
        .parse()
        .expect("parse uri");
    let response = client.get(uri).await.expect("request missing");
    assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);

    tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
        .await
        .expect("shutdown timed out")
        .expect("shutdown failed");

    let _ = fs::remove_dir_all(&fixture);
}

fn unique_static_dir(label: &str) -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("axm_web_static_{label}_{}", uuid::Uuid::new_v4()));
    dir
}

fn create_static_fixture(label: &str) -> PathBuf {
    let base = unique_static_dir(label);
    fs::create_dir_all(base.join("css")).expect("create css dir");
    fs::create_dir_all(base.join("js")).expect("create js dir");

    let index = r#"<!DOCTYPE html>
<html lang=\"en\">
  <head>
    <meta charset=\"utf-8\" />
    <title>Axiomind Poker</title>
    <link rel=\"stylesheet\" href=\"/static/css/app.css\" />
    <script src=\"/static/js/htmx.min.js\" defer></script>
  </head>
  <body>
    <main id=\"table\">Poker Table</main>
  </body>
</html>
"#;
    write_if_changed(base.join("index.html"), index);

    let css = r#":root {
  color-scheme: dark;
}

#table {
  display: grid;
  place-items: center;
  min-height: 100vh;
  background: radial-gradient(circle, #124, #012);
  table-layout: fixed;
}
"#;
    write_if_changed(base.join("css/app.css"), css);

    let js = r#"window.htmx = window.htmx || { version: 'test' };"#;
    write_if_changed(base.join("js/htmx.min.js"), js);

    base
}

fn write_if_changed(path: impl AsRef<Path>, contents: &str) {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write file");
}
