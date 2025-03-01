use sqlx::{Pool, Postgres, query_scalar};
use super::node::ChatMessage;

pub async  fn verify_user_workspace(workspace_id: u64, user_id: i32, db: Pool<Postgres>) -> bool {
    // Validate that the user owns the workspace containing the root_id
    query_scalar!(
            "SELECT EXISTS (SELECT 1 FROM workspaces WHERE id = $1 AND user_id = $2)",
            workspace_id,
            user_id
        )
        .fetch_one(&db)
        .await
        .unwrap_or(false)
}

pub async fn fetch_chat_id(workspace_id: u64, node_id: u64, db: Pool<Postgres>) -> Result<i32> {
    let chat_id: Option<i32> = if node_id == 0 {
        query_scalar!(
            "SELECT id FROM chats WHERE workspace_id = $1;",
            workspace_id
        ).fetch_optional(&db).await?;
    } else {
        query_scalar!(
            "SELECT id FROM chats WHERE workspace_id = $1 AND node_id = $2;",
            workspace_id,
            node_id
        ).fetch_optional(&db).await?;
    };
    
    let chat_id = if let Some(id) = chat_id {
        id
    } else {
        query_scalar!(
            "INSERT INTO chats (workspace_id, node_id) VALUES ($1, $2) RETURNING id;",
            workspace_id,
            if node_id == 0 { None } else { node_id }
        )
        .fetch_one(&db) // fetch_one ensures a value is returned
        .await?
    };

    Ok(chat_id)
}

pub async fn fetch_messages(chat_id: i32, db: Pool<Postgres>) -> Result<Vec<ChatMessage>> {
    let messages: Vec<String> = query_scalar!(
        "SELECT message FROM messages WHERE chat_id = $1 ORDER BY sent_at ASC;",
        chat_id
    )
    .fetch_all(&db)
    .await?;

    let mut messages_data: Vec<ChatMessage> = vec![];

    for i in messages {
        match serde_json::from_value(i) {
            Ok(message_raw) => {
                messages_data.push(message_raw);
            }
            Err(_e) => {}
        }
    }

    Ok(messages_data)
}

pub async fn workspace_tree_exists(workspace_id: u64, db: Pool<Postgres>) -> bool {
    let exists = query_scalar!(
        "SELECT root_id IS NOT NULL FROM workspaces WHERE id = $1;",
        workspace_id
    )
    .fetch_one(&db)
    .await;

    exists.unwrap_or(false)
}

pub async fn insert_message(chat_id: i32, message: ChatMessage, db: Pool<Postgres>) {
    match serde_json::to_string(&message) {
        Ok(message_data) => {
            query_scalar!(
                "INSERT INTO messages (message, chat_id, is_user) VALUES ($1, $2, $3);",
                message_data,
                chat_id,
                message.role == String::from("user")
            ).fetch_optional(&db).await;
        }
        Err(_e) => {
            return
        }
    }
}