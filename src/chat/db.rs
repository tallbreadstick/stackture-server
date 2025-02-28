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

pub async fn fetch_chat_id(workspace_id: u64, node_id: u64, db: Pool<Postgres>) -> Result<u64> {
    let chat_id = if node_id == 0 {
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
    
    if let None = chat_id {
        query_scalar!(
            "INSERT INTO chats (workspace_id, node_id) VALUES ($1, $2) RETURNING id;",
            workspace_id,
            if node_id == 0 {None} else {node_id}
        ).fetch_optional(&db).await?;
    }

    Ok(chat_id)
}

pub async fn fetch_messages(chat_id: u64, db: Pool<Postgres>) -> Result<Vec<ChatMessage>> {
    
}

pub async fn workspace_tree_exists(workspace_id: u64, db: Pool<Postgres>) -> bool {
    let exists = sqlx::query_scalar!(
        "SELECT root_id IS NOT NULL FROM workspaces WHERE id = $1;",
        workspace_id
    )
    .fetch_one(&db)
    .await;

    exists.unwrap_or(false)
}

pub async fn insert_message(chat_id: u64, message: ChatMessage, db: Pool<Postgres>) {

}