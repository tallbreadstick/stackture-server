use axum::{
    extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade}, response::IntoResponse
};
use serde::Deserialize;
use serde_json;

mod sapling;
mod node;

#[derive(Deserialize)]
struct ChatRequest {
    workspace_id: u64,
    node_id: u64,
    token: String
}

pub async fn websocket_listener(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    if let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let request: Result<ChatRequest, _> = serde_json::from_str(&text);
            
            match request {
                Ok(request_data) => {
                    // verify token here

                    let is_sapling: bool = true;

                    // do something with data passed

                    let _ = socket.send(Message::text("{\"status\": \"success\", \"message\": \"Session Opened\"}")).await;

                    if is_sapling {
                        sapling::sapling_chat(socket).await;
                    } else {
                        node::node_chat(socket).await;
                    }
                }
                Err(_e) => {
                    let _ = socket.send(Message::text("{\"status\": \"error\", \"message\": \"Incorrect request\"}")).await;
                }
            }
        }
    }
}