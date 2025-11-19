use anyhow::Result;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Events that can be sent to the web UI for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringEvent {
    /// A MIDI note was turned on
    NoteOn {
        note: u8,
        note_name: String,
        frequency: f32,
        velocity: u8,
        confidence: f32,
    },
    /// A MIDI note was turned off
    NoteOff { note: u8, note_name: String },
    /// Pitch bend was applied
    PitchBend { note: u8, bend_value: f32 },
    /// System status update
    Status { message: String },
    /// Recording started/stopped
    RecordingStatus { recording: bool },
}

/// Web server for monitoring the MIDI conversion process
pub struct WebServer {
    event_tx: broadcast::Sender<MonitoringEvent>,
    port: u16,
}

impl WebServer {
    /// Create a new web server
    pub fn new(port: u16) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self { event_tx, port }
    }

    /// Get a sender for broadcasting monitoring events
    pub fn event_sender(&self) -> broadcast::Sender<MonitoringEvent> {
        self.event_tx.clone()
    }

    /// Start the web server (runs in the background)
    pub async fn start(self) -> Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        info!("Starting web server on http://{}", addr);

        let app = Router::new()
            .route("/", get(index_handler))
            .route("/ws", get(ws_handler))
            .with_state(Arc::new(self.event_tx));

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        info!("Web UI available at http://{}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Serve the main HTML page
async fn index_handler() -> impl IntoResponse {
    Html(include_str!("../../static/index.html"))
}

/// WebSocket handler for real-time monitoring
async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(event_tx): axum::extract::State<Arc<broadcast::Sender<MonitoringEvent>>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, event_tx))
}

/// Handle WebSocket connection
async fn handle_socket(mut socket: WebSocket, event_tx: Arc<broadcast::Sender<MonitoringEvent>>) {
    debug!("WebSocket connection established");

    // Send initial status
    let status = MonitoringEvent::Status {
        message: "Connected to monitoring server".to_string(),
    };
    if let Ok(json) = serde_json::to_string(&status) {
        let _ = socket.send(Message::Text(json)).await;
    }

    // Subscribe to events
    let mut rx = event_tx.subscribe();

    // Forward events to the WebSocket
    while let Ok(event) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&event) {
            if socket.send(Message::Text(json)).await.is_err() {
                debug!("WebSocket connection closed");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_server_creation() {
        let server = WebServer::new(8080);
        assert!(server.port == 8080);
    }

    #[test]
    fn test_event_serialization() {
        let event = MonitoringEvent::NoteOn {
            note: 60,
            note_name: "C4".to_string(),
            frequency: 261.63,
            velocity: 80,
            confidence: 0.95,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("NoteOn"));
        assert!(json.contains("C4"));
    }

    #[test]
    fn test_event_sender() {
        let server = WebServer::new(8080);
        let sender = server.event_sender();

        // Subscribe to create a receiver
        let mut _rx = sender.subscribe();

        let event = MonitoringEvent::Status {
            message: "Test".to_string(),
        };

        // Should be able to send without error when there's a receiver
        let result = sender.send(event);
        assert!(result.is_ok());
    }
}
