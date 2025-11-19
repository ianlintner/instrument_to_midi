use instrument_to_midi::web::{MonitoringEvent, WebServer};
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_web_server_starts() {
    let server = WebServer::new(8082);
    let event_sender = server.event_sender();

    // Start server in background
    let server_handle = tokio::spawn(async move {
        // Server will run until the test ends
        let _ = server.start().await;
    });

    // Give server time to start
    sleep(Duration::from_millis(500)).await;

    // Test that we can send events
    let event = MonitoringEvent::Status {
        message: "Test server started".to_string(),
    };

    // Create a receiver to test
    let mut rx = event_sender.subscribe();

    // Send event
    let result = event_sender.send(event);
    assert!(result.is_ok());

    // Verify we can receive it
    let received = rx.try_recv();
    assert!(received.is_ok());

    // Cleanup
    server_handle.abort();
}

#[tokio::test]
async fn test_monitoring_events() {
    let server = WebServer::new(8083);
    let event_sender = server.event_sender();

    let mut rx = event_sender.subscribe();

    // Test different event types
    let events = vec![
        MonitoringEvent::NoteOn {
            note: 60,
            note_name: "C4".to_string(),
            frequency: 261.63,
            velocity: 80,
            confidence: 0.95,
        },
        MonitoringEvent::NoteOff {
            note: 60,
            note_name: "C4".to_string(),
        },
        MonitoringEvent::PitchBend {
            note: 60,
            bend_value: 0.5,
        },
        MonitoringEvent::Status {
            message: "Processing audio".to_string(),
        },
        MonitoringEvent::RecordingStatus { recording: true },
    ];

    for event in events {
        let result = event_sender.send(event);
        assert!(result.is_ok());
        let received = rx.try_recv();
        assert!(received.is_ok());
    }
}

#[test]
fn test_event_json_serialization() {
    // Test that all event types can be serialized to JSON
    let note_on = MonitoringEvent::NoteOn {
        note: 60,
        note_name: "C4".to_string(),
        frequency: 261.63,
        velocity: 80,
        confidence: 0.95,
    };
    let json = serde_json::to_string(&note_on).unwrap();
    assert!(json.contains("NoteOn"));
    assert!(json.contains("C4"));

    let note_off = MonitoringEvent::NoteOff {
        note: 60,
        note_name: "C4".to_string(),
    };
    let json = serde_json::to_string(&note_off).unwrap();
    assert!(json.contains("NoteOff"));

    let pitch_bend = MonitoringEvent::PitchBend {
        note: 60,
        bend_value: 0.5,
    };
    let json = serde_json::to_string(&pitch_bend).unwrap();
    assert!(json.contains("PitchBend"));

    let status = MonitoringEvent::Status {
        message: "Test".to_string(),
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("Status"));

    let recording = MonitoringEvent::RecordingStatus { recording: true };
    let json = serde_json::to_string(&recording).unwrap();
    assert!(json.contains("RecordingStatus"));
}
