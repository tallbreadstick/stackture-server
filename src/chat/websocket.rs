use axum::{
    extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade}, response::IntoResponse
};
use serde::Deserialize;
use serde_json;

mod sapling;
mod node;

#[derive(Deserialize)]
struct ChatRequest {
    id: u64,
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

                    if request_data.id == 0 {
                        // insert new session to db
                        // get insert_id
                    } else {
                        // check if it is a node or sapling (chat before the creation of tree)
                        // at error: send message and return
                    }

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