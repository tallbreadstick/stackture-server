use super::db::{fetch_chat_id, verify_user_workspace, workspace_tree_exists};
use super::node::{node_chat, ChatAIResponse, Node};
use crate::api::api::extract_token_data_str;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::{Pool, Postgres};

#[derive(Deserialize)]
struct ChatRequest {
    workspace_id: i32,
    node_id: i32,
    token: String,
}

pub enum WebSocketResponse<'a> {
    Success(String, Option<&'a Vec<Node>>),
    Error(WebSocketError),
}

impl<'a> WebSocketResponse<'a> {
    pub fn into_message(self) -> Message {
        match self {
            WebSocketResponse::Success(message, generated_tree) => Message::Text(
                serde_json::json!({
                    "status": "success",
                    "message": message,
                    "generateed_tree": generated_tree
                })
                .to_string()
                .into(),
            ),
            WebSocketResponse::Error(error) => Message::Text(
                serde_json::json!({
                    "status": "error",
                    "message": error.to_string()
                })
                .to_string()
                .into(),
            ),
        }
    }
}

#[derive(Display)]
pub enum WebSocketError {
    IncorrectRequest,
    UnauthorizedAccess,
    SessionCreationError,
    TokenError,
}

// !!! this might not be necessary anymore -> it is not used
pub fn create_socket_response(status: &str, message: &str) -> String {
    let mut content = ChatAIResponse::default();
    content.status = status.to_string();
    content.message = message.to_string();
    return serde_json::to_string(&content).unwrap_or("".to_string());
}

pub async fn websocket_listener(
    ws: WebSocketUpgrade,
    State(db): State<Pool<Postgres>>,
) -> impl IntoResponse {
    let _ = ws.on_upgrade(move |socket| handle_socket(socket, db.clone()));
}

async fn handle_socket(mut socket: WebSocket, db: Pool<Postgres>) {
    let Some(Ok(msg)) = socket.recv().await else { return };
    let Message::Text(text) = msg else { return };

    let request_data: ChatRequest = match serde_json::from_str(&text) {
        Ok(data) => data,
        Err(_) => {
            socket
                .send(
                    WebSocketResponse::Error(WebSocketError::IncorrectRequest)
                        .into_message(),
                )
                .await
                .unwrap_or_else(|_| {
                    // log error?
                });
            return;
        }
    };

    let success_token = match extract_token_data_str(request_data.token) {
        Ok(token) => token,
        Err(_) => {
            socket
                .send(
                    WebSocketResponse::Error(WebSocketError::TokenError)
                        .into_message(),
                )
                .await
                .unwrap_or_else(|_| {
                    // log error?
                });
            return;
        }
    };

    if !verify_user_workspace(request_data.workspace_id, success_token.user_id, db.clone()).await {
        socket
            .send(
                WebSocketResponse::Error(WebSocketError::UnauthorizedAccess)
                    .into_message(),
            )
            .await
            .unwrap_or_else(|_| {
                // log error?
            });
        return;
    }

    let tree_exist = workspace_tree_exists(request_data.workspace_id, db.clone()).await;

    let chat_id = match fetch_chat_id(request_data.workspace_id, request_data.node_id, db.clone())
        .await
    {
        Ok(id) => id,
        Err(_) => {
            socket
                .send(
                    WebSocketResponse::Error(WebSocketError::SessionCreationError)
                        .into_message(),
                )
                .await
                .unwrap_or_else(|_| {
                    // log error?
                });
            return;
        }
    };

    socket
        .send(
            WebSocketResponse::Success("Session Opened".into(), None)
                .into_message(),
        )
        .await
        .unwrap_or_else(|_| {
            // log error?
        });

    node_chat(socket, tree_exist, chat_id, db.clone()).await;
}

// async fn handle_socket(mut socket: WebSocket, db: Pool<Postgres>) {
//     if let Some(Ok(msg)) = socket.recv().await {
//         if let Message::Text(text) = msg {
//             let request: Result<ChatRequest, _> = serde_json::from_str(&text);

//             match request {
//                 Ok(request_data) => {
//                     match extract_token_data_str(request_data.token) {
//                         Ok(success_token) => {
//                             if !verify_user_workspace(
//                                 request_data.workspace_id,
//                                 success_token.user_id,
//                                 db.clone(),
//                             )
//                             .await
//                             {
//                                 let _ = socket
//                                     .send(
//                                         WebSocketResponse::Error(
//                                             WebSocketError::UnauthorizedAccess,
//                                         )
//                                         .into_message(),
//                                     )
//                                     .await;
//                                 return;
//                             }
//                         }
//                         Err(_) => {
//                             let _ = socket
//                                 .send(
//                                     WebSocketResponse::Error(WebSocketError::TokenError)
//                                         .into_message(),
//                                 )
//                                 .await;
//                             return;
//                         }
//                     };

//                     let tree_exist: bool =
//                         workspace_tree_exists(request_data.workspace_id, db.clone()).await;
//                     let chat_id: i32;

//                     match fetch_chat_id(request_data.workspace_id, request_data.node_id, db.clone())
//                         .await
//                     {
//                         Ok(id) => {
//                             chat_id = id;
//                         }
//                         Err(_e) => {
//                             let _ = socket
//                                 .send(
//                                     WebSocketResponse::Error(WebSocketError::SessionCreationError)
//                                         .into_message(),
//                                 )
//                                 .await;
//                             return;
//                         }
//                     };

//                     let _ = socket
//                         .send(
//                             WebSocketResponse::Success("Session Opened".into(), None)
//                                 .into_message(),
//                         )
//                         .await;

//                     node_chat(socket, tree_exist, chat_id, db.clone()).await;
//                 }
//                 Err(_e) => {
//                     let _ = socket
//                         .send(
//                             WebSocketResponse::Error(WebSocketError::IncorrectRequest)
//                                 .into_message(),
//                         )
//                         .await;
//                 }
//             }
//         }
//     }
// }
