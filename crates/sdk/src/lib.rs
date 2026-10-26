#![forbid(unsafe_code)]
//! Optional lightweight telemetry and synthetic input bridge crate (`ui-qa-sdk`, `M2.4`)
//! for Rust native GUI applications (`iced`, `egui`, `Slint`).
//!
//! Target applications can embed `QaSdkServer` behind a feature flag to expose their internal
//! widget tree and accept background synthetic clicks/text input without OS focus changes.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkWidgetNode {
    pub id: String,
    pub label: String,
    pub role: String,
    pub value: Option<String>,
    pub enabled: bool,
    pub bounds: [i32; 4], // [x, y, width, height]
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum SdkCommand {
    Ping,
    GetWidgets,
    Click { id: Option<String>, x: i32, y: i32 },
    InputText { id: String, text: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SdkResponse {
    Pong { protocol_version: u32 },
    Widgets { nodes: Vec<SdkWidgetNode> },
    Ack { message: String },
    Error { message: String },
}

pub fn sdk_protocol_version() -> u32 {
    1
}

/// Trait implemented by an `egui`, `iced`, or `Slint` application to expose widgets and handle synthetic actions.
pub trait QaAppAdapter: Send + Sync + 'static {
    fn snapshot_widgets(&self) -> Vec<SdkWidgetNode>;
    fn on_synthetic_click(&self, id: Option<&str>, x: i32, y: i32) -> Result<String, String>;
    fn on_synthetic_input(&self, id: &str, text: &str) -> Result<String, String>;
}

pub struct QaSdkServer {
    local_addr: std::net::SocketAddr,
    shutdown: Arc<std::sync::atomic::AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
}

impl QaSdkServer {
    /// Starts a lightweight localhost IPC listener on an ephemeral port (or requested port).
    pub fn start<A: QaAppAdapter>(bind_addr: &str, adapter: Arc<A>) -> std::io::Result<Self> {
        let listener = TcpListener::bind(bind_addr)?;
        listener.set_nonblocking(true)?;
        let local_addr = listener.local_addr()?;
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);

        let handle = thread::spawn(move || {
            while !shutdown_clone.load(std::sync::atomic::Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                        let _ = handle_client(stream, adapter.as_ref());
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(15));
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            local_addr,
            shutdown,
            join_handle: Some(handle),
        })
    }

    pub fn local_addr(&self) -> std::net::SocketAddr {
        self.local_addr
    }

    pub fn stop(&mut self) {
        self.shutdown
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(h) = self.join_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for QaSdkServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn handle_client<A: QaAppAdapter>(mut stream: TcpStream, adapter: &A) -> std::io::Result<()> {
    let mut line = String::new();
    {
        let mut reader = BufReader::new(&stream);
        reader.read_line(&mut line)?;
    }
    let resp = match serde_json::from_str::<SdkCommand>(line.trim()) {
        Ok(SdkCommand::Ping) => SdkResponse::Pong {
            protocol_version: sdk_protocol_version(),
        },
        Ok(SdkCommand::GetWidgets) => SdkResponse::Widgets {
            nodes: adapter.snapshot_widgets(),
        },
        Ok(SdkCommand::Click { id, x, y }) => {
            match adapter.on_synthetic_click(id.as_deref(), x, y) {
                Ok(msg) => SdkResponse::Ack { message: msg },
                Err(e) => SdkResponse::Error { message: e },
            }
        }
        Ok(SdkCommand::InputText { id, text }) => match adapter.on_synthetic_input(&id, &text) {
            Ok(msg) => SdkResponse::Ack { message: msg },
            Err(e) => SdkResponse::Error { message: e },
        },
        Err(e) => SdkResponse::Error {
            message: format!("Invalid SDK JSON command: {e}"),
        },
    };

    let mut out = serde_json::to_string(&resp).unwrap_or_default();
    out.push('\n');
    stream.write_all(out.as_bytes())?;
    stream.flush()
}

pub fn send_sdk_command(
    addr: std::net::SocketAddr,
    cmd: &SdkCommand,
) -> std::io::Result<SdkResponse> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut payload = serde_json::to_string(cmd)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    payload.push('\n');
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    serde_json::from_str::<SdkResponse>(line.trim())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Built-in in-memory mock adapter for testing `egui`/`iced`/`Slint` SDK integration.
#[derive(Default)]
pub struct InMemoryAppAdapter {
    pub widgets: Mutex<Vec<SdkWidgetNode>>,
    pub click_history: Mutex<Vec<(Option<String>, i32, i32)>>,
}

impl QaAppAdapter for InMemoryAppAdapter {
    fn snapshot_widgets(&self) -> Vec<SdkWidgetNode> {
        self.widgets.lock().map(|g| g.clone()).unwrap_or_default()
    }

    fn on_synthetic_click(&self, id: Option<&str>, x: i32, y: i32) -> Result<String, String> {
        if let Ok(mut hist) = self.click_history.lock() {
            hist.push((id.map(str::to_string), x, y));
        }
        Ok(format!("Clicked {:?} at ({x}, {y})", id))
    }

    fn on_synthetic_input(&self, id: &str, text: &str) -> Result<String, String> {
        Ok(format!("Input '{text}' into '{id}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_server_and_client_roundtrip() {
        let adapter = Arc::new(InMemoryAppAdapter::default());
        adapter.widgets.lock().expect("lock").push(SdkWidgetNode {
            id: "btn_cal".to_string(),
            label: "Calibrate".to_string(),
            role: "Button".to_string(),
            value: None,
            enabled: true,
            bounds: [20, 40, 140, 36],
        });

        let mut server =
            QaSdkServer::start("127.0.0.1:0", Arc::clone(&adapter)).expect("start server");
        let addr = server.local_addr();

        let pong = send_sdk_command(addr, &SdkCommand::Ping).expect("ping");
        assert_eq!(
            pong,
            SdkResponse::Pong {
                protocol_version: 1
            }
        );

        let widgets = send_sdk_command(addr, &SdkCommand::GetWidgets).expect("get widgets");
        match widgets {
            SdkResponse::Widgets { nodes } => assert_eq!(nodes.len(), 1),
            other => panic!("unexpected response: {other:?}"),
        }

        let click = send_sdk_command(
            addr,
            &SdkCommand::Click {
                id: Some("btn_cal".to_string()),
                x: 90,
                y: 58,
            },
        )
        .expect("click");
        assert!(matches!(click, SdkResponse::Ack { .. }));
        assert_eq!(adapter.click_history.lock().expect("lock").len(), 1);

        server.stop();
    }
}
