mod config;
use config::SETTINGS;
mod messages;
use messages::*;


use axum::{extract::{
    ws::{Message, WebSocket},
    WebSocketUpgrade,
}, response::{Html, IntoResponse}, routing::get, Router};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as TungsteniteMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use base64::Engine;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const OPENAI_REALTIME_URL: &str =
    "wss://api.openai.com/v1/realtime?model=gpt-4o-realtime-preview-2024-10-01";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing (logging)
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Ensure settings are loaded (this will panic if OPENAI_API_KEY is missing)
    let _ = &SETTINGS.openai_api_key;
    tracing::info!("OpenAI API key loaded.");

    let app = Router::new()
        .route("/", get(get_index))
        .route("/ws", get(websocket_handler));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Serve the index.html file
async fn get_index() -> impl IntoResponse {
    match tokio::fs::read_to_string("index.html").await {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read index.html: {}", e),
        )
            .into_response(),
    }
}

/// Handle incoming WebSocket connections from clients
async fn websocket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_client_socket)
}

/// Main logic for handling a single client WebSocket
async fn handle_client_socket(mut client_ws: WebSocket) {
    tracing::info!("Client connected");

    // Send ready signal
    let ready_msg = serde_json::json!({"type": "ready"});
    if client_ws
        .send(Message::Text(ready_msg.to_string()))
        .await
        .is_err()
    {
        tracing::error!("Failed to send ready message to client");
        return;
    }

    // Main loop: wait for messages from the client
    while let Some(msg) = client_ws.recv().await {
        match msg {
            Ok(Message::Binary(audio_data)) => {
                tracing::info!("Received audio data: {} bytes", audio_data.len());
                if audio_data.is_empty() {
                    tracing::warn!("Audio data is empty!");
                    let _ = client_ws
                        .send(Message::Text(
                            serde_json::json!({"type": "error", "message": "Audio data is empty"})
                                .to_string(),
                        ))
                        .await;
                    continue;
                }

                // Spawn a new task to handle the OpenAI connection
                // This allows the server to process other messages from the client
                // if needed, though in this design we process inline for simplicity.
                if let Err(e) = handle_openai_stream(&mut client_ws, audio_data).await {
                    tracing::error!("OpenAI stream error: {:?}", e);
                    let _ = client_ws
                        .send(Message::Text(
                            serde_json::json!({"type": "error", "message": e.to_string()})
                                .to_string(),
                        ))
                        .await;
                }
            }
            Ok(Message::Text(text)) => {
                // Handle text messages (e.g., pings)
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                    if v.get("type").and_then(|t| t.as_str()) == Some("ping") {
                        let _ = client_ws
                            .send(Message::Text(
                                serde_json::json!({"type": "pong"}).to_string(),
                            ))
                            .await;
                    }
                }
            }
            Ok(Message::Close(_)) => {
                tracing::info!("Client disconnected");
                break;
            }
            Err(e) => {
                tracing::error!("Client WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

/// Connects to OpenAI, sends audio, and streams the response back to the client
async fn handle_openai_stream(
    client_ws: &mut WebSocket,
    audio_data: Vec<u8>,
) -> anyhow::Result<()> {
    tracing::info!("Connecting to OpenAI Realtime API...");

    // Create the connection request with headers
    let mut request = OPENAI_REALTIME_URL.into_client_request()?;
    let headers = request.headers_mut();
    headers.insert(
        "Authorization",
        format!("Bearer {}", SETTINGS.openai_api_key).parse()?,
    );
    headers.insert("OpenAI-Beta", "realtime=v1".parse()?);

    // Connect to OpenAI
    let (openai_ws, _) = connect_async(request).await?;
    tracing::info!("Connected to OpenAI Realtime API");

    let (mut openai_write, mut openai_read) = openai_ws.split();

    // 1. Send session configuration
    let session_update = SessionUpdate::new();
    openai_write
        .send(TungsteniteMessage::Text(
            serde_json::to_string(&session_update)?,
        ))
        .await?;

    // 2. Send audio data
    let audio_base64 = base64::engine::general_purpose::STANDARD.encode(&audio_data);
    tracing::debug!(
        "Sending audio buffer: {} chars base64 ({} bytes raw)",
        audio_base64.len(),
        audio_data.len()
    );
    let audio_append = AudioAppend::new(audio_base64);
    openai_write
        .send(TungsteniteMessage::Text(
            serde_json::to_string(&audio_append)?,
        ))
        .await?;

    // 3. Commit the audio buffer
    let commit = Commit::default();
    tracing::debug!("Committing audio buffer: {:?}", commit);
    openai_write
        .send(TungsteniteMessage::Text(serde_json::to_string(&commit)?))
        .await?;

    // 4. Create a response
    let response_create = ResponseCreate::default();
    openai_write
        .send(TungsteniteMessage::Text(
            serde_json::to_string(&response_create)?,
        ))
        .await?;

    // 5. Stream responses back to client
    let mut transcript_chunks = Vec::new();

    while let Some(msg) = openai_read.next().await {
        match msg {
            Ok(TungsteniteMessage::Text(text)) => {
                let event: OpenAIEvent = serde_json::from_str(&text)?;
                tracing::debug!("Received event: {}", event.event_type);

                // Forward all events to client for transparency
                client_ws.send(Message::Text(text)).await?;

                // Collect transcript
                if event.event_type == "response.audio_transcript.delta" {
                    if let Some(delta) = event.delta {
                        transcript_chunks.push(delta);
                    }
                }

                // When response is done, signal completion
                if event.event_type == "response.done" {
                    tracing::info!("Response complete");
                    let complete_msg = serde_json::json!({
                        "type": "response_complete",
                        "transcript": transcript_chunks.join("")
                    });
                    client_ws.send(Message::Text(complete_msg.to_string())).await?;
                    break;
                }
            }
            Ok(TungsteniteMessage::Close(_)) => {
                tracing::info!("OpenAI connection closed");
                break;
            }
            Err(e) => {
                tracing::warn!("OpenAI WebSocket error: {}", e);
                return Err(e.into());
            }
            _ => { /* Ignore other message types */ }
        }
    }

    tracing::info!("OpenAI stream finished.");
    Ok(())
}
