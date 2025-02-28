use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::{Pool, Postgres};
use crate::api::api::extract_token_data_str;

use super::node::{node_chat, ChatAIResponse};
use super::db::{verify_user_workspace, fetch_chat_id, workspace_tree_exists};

#[derive(Deserialize)]
struct ChatRequest {
    workspace_id: u64,
    node_id: u64,
    token: String
}

pub fn create_socket_response(status: &str, message: &str) -> String {
    let mut content = ChatAIResponse::default();
    content.status = status.to_string();
    content.message = message.to_string();

    return serde_json::to_string(&content).unwrap_or("".to_string());
}

pub async fn websocket_listener(ws: WebSocketUpgrade, State(db): State<Pool<Postgres>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, db.clone()));
}

async fn handle_socket(mut socket: WebSocket, db: Pool<Postgres>) {
    if let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let request: Result<ChatRequest, _> = serde_json::from_str(&text);
            
            match request {
                Ok(request_data) => {

                    match extract_token_data_str(request_data.token) {
                        Ok(success_token) => {
                            if !verify_user_workspace(request_data.workspace_id, success_token.user_id, db).await {
                                let _ = socket.send(Message::text(create_socket_response("error", "Unauthorized Access"))).await;
                                return;
                            }
                        }
                        Err(_) => {
                            let _ = socket.send(Message::text(create_socket_response("error", "Token Error"))).await;
                            return;
                        }
                    };

                    let tree_exist: bool = workspace_tree_exists(request_data.workspace_id, db).await;
                    let chat_id: u64;
                    
                    match fetch_chat_id(request_data.workspace_id, request_data.node_id, db).await {
                        Ok(id) => {
                            chat_id = id;
                        }
                        Err(_e) => {
                            let _ = socket.send(Message::text(create_socket_response("error", "Session Creation error"))).await;
                            return;
                        }
                    };

                    let _ = socket.send(Message::text(create_socket_response("success", "Session Opened"))).await;

                    node_chat(socket, tree_exist, chat_id, db).await;
                }
                Err(_e) => {
                    let _ = socket.send(Message::text(create_socket_response("error", "Incorrect Request"))).await;
                }
            }
        }
    }
}