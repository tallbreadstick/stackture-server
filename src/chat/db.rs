use std::collections::HashMap;

use sqlx::{query_as, query_scalar, Error, Pool, Postgres, Transaction};
use super::node::{ChatMessage, Node};


struct DBNode {
    id: i32,
    name: String,
    summary: Option<String>,
    optional: Option<bool>,
    resolved: Option<bool>,
    icon: Option<String>,
    parents: Option<Vec<i32>>,
    branches: Option<Vec<i32>>,
}


pub async  fn verify_user_workspace(workspace_id: i32, user_id: i32, db: Pool<Postgres>) -> bool {
    // Validate that the user owns the workspace containing the root_id
    query_scalar!(
            "SELECT EXISTS (SELECT 1 FROM workspaces WHERE id = $1 AND user_id = $2)",
            workspace_id,
            user_id
        )
        .fetch_one(&db)
        .await
        .unwrap_or(Some(false)).unwrap_or(false)
}

pub async fn fetch_chat_id(workspace_id: i32, node_id: i32, db: Pool<Postgres>) -> Result<i32, ()> {
    let chat_id: Option<i32> = if node_id == 0 {
        query_scalar!(
            "SELECT id FROM chats WHERE workspace_id = $1;",
            workspace_id
        )
        .fetch_optional(&db)
        .await
        .map_err(|_| ())?
    } else {
        query_scalar!(
            "SELECT id FROM chats WHERE workspace_id = $1 AND node_id = $2;",
            workspace_id,
            node_id
        )
        .fetch_optional(&db)
        .await
        .map_err(|_| ())?
    };
    
    let chat_id = if let Some(id) = chat_id {
        id
    } else {
        query_scalar!(
            "INSERT INTO chats (workspace_id, node_id) VALUES ($1, $2) RETURNING id;",
            workspace_id,
            if node_id == 0 { None } else { Some(node_id) }
        )
        .fetch_one(&db) // fetch_one ensures a value is returned
        .await
        .map_err(|_| ())?
    };

    Ok(chat_id)
}

pub async fn fetch_current_tree(workspace_id: i32, db: &Pool<Postgres>) -> Result<Vec<Node>, ()> {
    query_as!(
        DBNode,
        "SELECT 
            id,
            name,
            summary,
            optional,
            resolved,
            icon,
            (SELECT array_agg(parent_id) FROM node_parents WHERE node_id = id) AS parents,
            (SELECT array_agg(node_id) FROM node_parents WHERE parent_id = id) AS branches
        FROM nodes WHERE workspace_id = $1",
        workspace_id
    )
    .fetch_all(db)
    .await
    .map_err(|_| ())
    .map(|n| {
        let mut nodes: Vec<Node> = vec![];

        for i in n {
            nodes.push(Node {
                id: i.id,
                name: i.name,
                summary: i.summary.unwrap_or("".into()),
                icon: i.icon.unwrap_or("".into()),
                parents: i.parents.unwrap_or(Vec::new()),
                branches: i.branches.unwrap_or(Vec::new()),
                optional: i.optional.unwrap_or(false),
                resolved: i.resolved.unwrap_or(false)
            });
        }

        return nodes;
    })
}

pub async fn fetch_messages(chat_id: i32, db: Pool<Postgres>) -> Result<Vec<ChatMessage>, ()> {
    let messages: Vec<String> = query_scalar!(
        "SELECT message FROM (SELECT message, sent_at FROM messages WHERE chat_id = $1 ORDER BY sent_at DESC LIMIT 6) sub ORDER BY sent_at ASC;",
        chat_id
    )
    .fetch_all(&db)
    .await
    .map_err(|_| ())?
    .iter()
    .filter(|s| s.is_some())
    .map(|s| s.clone().unwrap_or_default())
    .collect();

    let mut messages_data: Vec<ChatMessage> = vec![];

    for i in messages {
        match serde_json::from_str(&i) {
            Ok(message_raw) => {
                messages_data.push(message_raw);
            }
            Err(_e) => {}
        }
    }

    Ok(messages_data)
}

pub async fn workspace_tree_exists(workspace_id: i32, db: Pool<Postgres>) -> bool {
    let exists = query_scalar!(
        "SELECT root_id IS NOT NULL FROM workspaces WHERE id = $1;",
        workspace_id
    )
    .fetch_one(&db)
    .await;

    exists.unwrap_or(Some(false)).unwrap_or(false)
}

pub async fn insert_message(chat_id: i32, message: &ChatMessage, db: &Pool<Postgres>) {
    match serde_json::to_string(message) {
        Ok(message_data) => {
            let _ = query_scalar!(
                "INSERT INTO messages (message, chat_id, is_user) VALUES ($1, $2, $3);",
                message_data,
                chat_id,
                message.role == String::from("user")
            ).fetch_optional(db).await;
        }
        _ => {}
    }
}

pub async fn insert_tree(workspace_id: i32, tree: &mut Vec<Node>, db: &Pool<Postgres>) -> Result<(), Error> {
    let tx: Transaction<Postgres> = db.begin().await?;

    if let Err(_) = query_scalar!("DELETE FROM nodes WHERE workspace_id = $1;", workspace_id).fetch_optional(db).await {
        // Error cannot remove the current tree
        tx.rollback().await?;
        return Err(Error::PoolClosed);
    }

    let mut keys: HashMap<i32, i32> = HashMap::new();

    // i dont like the method, but hell yeahhh
    for i in tree.as_mut_slice() {
        match query_scalar!(
            "INSERT INTO nodes (workspace_id, name, summary, optional, resolved, icon) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id;",
            workspace_id,
            i.name,
            i.summary,
            i.optional,
            i.resolved,
            i.icon
        ).fetch_one(db).await {
            Ok(id) => {
                keys.insert(i.id, id);
            }
            Err(_) => {
                // Error cannot insert node
                tx.rollback().await?;
                return Err(Error::PoolClosed);
            }
        }
    }

    for i in tree {
        for parent in i.parents.as_mut_slice() {
            if let Err(_) = query_scalar!(
                "INSERT INTO node_parents (node_id, parent_id) VALUES ($1, $2);",
                keys.get(&i.id).unwrap(),    // :D value should be expected from above... unless some bit in the system is being a good boy
                keys.get(parent).unwrap()
            ).fetch_optional(db).await {
                // Error parent insertion
                tx.rollback().await?;
                return Err(Error::PoolClosed);
            }

            *parent = *keys.get_mut(parent).unwrap();
        }

        for branch in i.branches.as_mut_slice() {
            *branch = *keys.get_mut(branch).unwrap();
        }

        i.id = *keys.get(&i.id).unwrap();
    }

    tx.commit().await?;

    Ok(())
}