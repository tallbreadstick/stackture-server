use axum::{extract::{Path, State}, Json};
use axum_extra::{headers::{authorization::Bearer, Authorization}, TypedHeader};
use sqlx::{Pool, Postgres};
use crate::api::api::ApiError;
use serde::{Serialize, Deserialize};
use crate::api::api::extract_token_data;
use super::db::{fetch_messages, fetch_chat_id, verify_user_workspace};

#[derive(Deserialize, Serialize)]
pub struct Message {
    pub message: String,
    pub is_user: bool
}

pub async fn fetch_chat(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path((workspace_id, node_id)): Path<(i32, i32)>,
) -> Result<Json<Vec<Message>>, ApiError> {
    match extract_token_data(auth) {
        Ok(token_data) => {
            if !verify_user_workspace(workspace_id, token_data.user_id, db.clone()).await {
                return Err(ApiError::UnauthorizedAccess);
            }
        }
        Err(e) => {
            return Err(e);
        }
    }

    if let Ok(chat_id) = fetch_chat_id(workspace_id, node_id, db.clone()).await {
        if let Ok(chats) = fetch_messages(chat_id, db).await {
            let mut chat_responses: Vec<Message> = vec![];

            for x in chats {
                if let Some(_) = x.tool_calls {
                    chat_responses.push(Message {
                        message: "Here is the generated tree.".into(),
                        is_user: false
                    });
                }
                chat_responses.push(Message {
                    message: x.content.unwrap_or("".into()),
                    is_user: x.role == "user"
                });
            }

            if chat_responses.len() > 0 {
                return Ok(Json(chat_responses));
            }
        }
    }

    return Err(ApiError::ItemNotFound);
}