use axum::{http::StatusCode, response::IntoResponse};
use serde::Serialize;
use sqlx::{Pool, Postgres};

#[derive(Serialize)]
pub enum NodeOperationError {
    NonexistentNode, // returned if a node being operated on does not exist
    RootAlreadyExists, // returned if a root node already exists on CREATE
    ForbiddenLink, // returned if the user attempts to link nodes from different workspaces
    CyclicReference, // returned if the user attempts a bad BORROW or TAKE on a node to a parent or ancestor
    DatabaseOperationFailed
}

impl IntoResponse for NodeOperationError {
    fn into_response(self) -> axum::response::Response {
        match self {
            NodeOperationError::NonexistentNode => {
                (StatusCode::NOT_FOUND, "NonexistentNode").into_response()
            },
            NodeOperationError::RootAlreadyExists => {
                (StatusCode::CONFLICT, "RootAlreadyExists").into_response()
            },
            NodeOperationError::ForbiddenLink => {
                (StatusCode::FORBIDDEN, "ForbiddenLink").into_response()
            },
            NodeOperationError::CyclicReference => {
                (StatusCode::BAD_REQUEST, "CyclicReference").into_response()
            },
            NodeOperationError::DatabaseOperationFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "DatabaseOperationFailed").into_response()
            }
        }
    }
}

/*

    TREE OPERATION RULES

    There are 6 primitive operations:

    CREATE node                     -- Creates the root node. Fails if root already exists.
    ADD node                        -- Creates a branch on a node.
    BORROW branch TO node           -- Links* 'branch' as a child to 'node'.
    DROP branch FROM node           -- Unlinks 'branch' from being a child to 'node'.
    TAKE branch TO node             -- Links 'branch' as a child to 'node' and also evicts all previous parents of 'branch'.
    DELETE node                     -- Deletes a node. Cascades deletion to all descendants which become orphaned in the process.

    There are rules to be followed when performing these operations:

        - You cannot operate on non-existent nodes.
        - You cannot create a root that already exists.
        - You cannot borrow or take a branch that is your node's ancestor.
        - You cannot link nodes across different workspaces.
        - *When you borrow a branch to a node, all ancestors of your node must drop the branch.

*/

// Create a root node for a workspacea
pub async fn create_node(
    workspace: i32,
    name: &str,
    summary: &str,
    db: &Pool<Postgres>
) -> Result<i32, NodeOperationError> {
    let mut tx = db.begin().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    let existing_root: Option<(i32,)> = sqlx::query_as(
        "SELECT id FROM nodes WHERE workspace = $1 AND parent IS NULL"
    )
    .bind(workspace)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    if existing_root.is_some() {
        return Err(NodeOperationError::RootAlreadyExists);
    }
    let node_id: i32  = sqlx::query_scalar(
        "INSERT INTO nodes (workspace, name, summary, parent) VALUES ($1, $2, $3, NULL) RETURNING id"
    )
    .bind(workspace)
    .bind(name)
    .bind(summary)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    let _ = sqlx::query(
        "INSERT INTO chats (node_id, workspace_id) VALUES ($1, $2)"
    )
    .bind(node_id)
    .bind(workspace)
    .execute(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    tx.commit().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    Ok(node_id)
}

// Spawn a branch onto a node
pub async fn add_node(
    workspace: i32,
    node: i32,
    name: &str,
    summary: &str,
    db: &Pool<Postgres>
) -> Result<i32, NodeOperationError> {
    let mut tx = db.begin().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    let parent_exists: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM nodes WHERE id = $1 AND workspace = $2"
    )
    .bind(node)
    .bind(workspace)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    if parent_exists.is_none() {
        return Err(NodeOperationError::NonexistentNode);
    }    
    let node_id: i32 = sqlx::query_scalar(
        "INSERT INTO nodes (workspace, name, summary, parent) VALUES ($1, $2, $3, $4) RETURNING id"
    )
    .bind(workspace)
    .bind(name)
    .bind(summary)
    .bind(node)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    let _ = sqlx::query(
        "INSERT INTO chats (node_id, workspace_id) VALUES ($1, $2)"
    )
    .bind(node_id)
    .bind(workspace)
    .execute(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    tx.commit().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    Ok(node_id)
}

// Borrow a node
pub async fn borrow_node(
    node: i32,
    branch: i32,
    db: &Pool<Postgres>
) -> Result<(), NodeOperationError> {
    let mut tx = db.begin().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    // 1️⃣ Check if 'branch' is an ancestor of 'node' using level-order traversal
    let mut queue = vec![branch];

    while let Some(current) = queue.pop() {
        let children: Vec<i32> = sqlx::query_scalar(
            "SELECT node_id FROM node_parents WHERE parent_id = $1"
        )
        .bind(current)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

        if children.contains(&node) {
            return Err(NodeOperationError::CyclicReference);
        }

        queue.extend(children);
    }

    // 2️⃣ Find all ancestors of 'node' (to remove 'branch' from them)
    let mut ancestors = vec![];
    let mut current = node;

    while let Some(parent) = sqlx::query_scalar(
        "SELECT parent_id FROM node_parents WHERE node_id = $1"
    )
    .bind(current)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?
    {
        if let Some(parent_id) = parent {
            ancestors.push(parent_id);
            current = parent_id;
        } else {
            break;
        }
    }

    // 3️⃣ Drop 'branch' from all ancestors
    if !ancestors.is_empty() {
        sqlx::query(
            "DELETE FROM node_parents WHERE parent_id = ANY($1) AND node_id = $2"
        )
        .bind(&ancestors)
        .bind(branch)
        .execute(&mut *tx)
        .await
        .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    }

    // 4️⃣ Borrow 'branch' to 'node'
    sqlx::query(
        "INSERT INTO node_parents (node_id, parent_id) VALUES ($1, $2)"
    )
    .bind(branch)
    .bind(node)
    .execute(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    // 5️⃣ Commit transaction
    tx.commit().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    Ok(())
}

pub async fn drop_node(
    node: i32,
    branch: i32,
    db: &Pool<Postgres>
) -> Result<(), NodeOperationError> {
    let mut tx = db.begin().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    // Step 1: Ensure both node and branch exist
    let exists: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM nodes WHERE id = $1 OR id = $2"
    )
    .bind(node)
    .bind(branch)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    if exists.is_none() {
        return Err(NodeOperationError::NonexistentNode);
    }

    // Step 2: Check if node is actually a parent of branch
    let is_parent: Option<i32> = sqlx::query_scalar(
        "SELECT parent_id FROM node_parents WHERE node_id = $1 AND parent_id = $2"
    )
    .bind(branch)
    .bind(node)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    if is_parent.is_none() {
        return Ok(()); // Nothing to do, branch is not actually a child of node
    }

    // Step 3: Remove the parent-child relationship
    sqlx::query(
        "DELETE FROM node_parents WHERE node_id = $1 AND parent_id = $2"
    )
    .bind(branch)
    .bind(node)
    .execute(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    // Step 4: Check if branch has any remaining parents
    let remaining_parents: Option<i32> = sqlx::query_scalar(
        "SELECT parent_id FROM node_parents WHERE node_id = $1 LIMIT 1"
    )
    .bind(branch)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    // Step 5: If branch has no remaining parents, delete it and cascade delete
    if remaining_parents.is_none() {
        delete_node_bfs(branch, &mut tx).await?;
    }

    tx.commit().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    Ok(())
}

// Recursive function to delete node and its orphaned descendants
async fn delete_node_bfs(
    root: i32,
    tx: &mut sqlx::Transaction<'_, Postgres>
) -> Result<(), NodeOperationError> {
    let mut queue = vec![root];

    while let Some(node) = queue.pop() {
        // Fetch all children of the node
        let children: Vec<i32> = sqlx::query_scalar(
            "SELECT node_id FROM node_parents WHERE parent_id = $1"
        )
        .bind(node)
        .fetch_all(&mut **tx)
        .await
        .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

        // Add children to queue for later deletion
        queue.extend(children);

        // Delete node's relationships first (to avoid foreign key constraint issues)
        sqlx::query("DELETE FROM node_parents WHERE node_id = $1 OR parent_id = $1")
            .bind(node)
            .execute(&mut **tx)
            .await
            .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

        // Delete the node itself
        sqlx::query("DELETE FROM nodes WHERE id = $1")
            .bind(node)
            .execute(&mut **tx)
            .await
            .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    }

    Ok(())
}

pub async fn take_node(
    node: i32,
    branch: i32,
    db: &Pool<Postgres>
) -> Result<(), NodeOperationError> {
    let mut tx = db.begin().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    // 1️⃣ Check if 'branch' is an ancestor of 'node' using level-order traversal
    let mut queue = vec![branch];

    while let Some(current) = queue.pop() {
        let children: Vec<i32> = sqlx::query_scalar(
            "SELECT node_id FROM node_parents WHERE parent_id = $1"
        )
        .bind(current)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

        if children.contains(&node) {
            return Err(NodeOperationError::CyclicReference);
        }

        queue.extend(children);
    }

    // 2️⃣ Remove all existing parent links of 'branch'
    sqlx::query(
        "DELETE FROM node_parents WHERE node_id = $1"
    )
    .bind(branch)
    .execute(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    // 3️⃣ Assign 'branch' to 'node' as its new parent
    sqlx::query(
        "INSERT INTO node_parents (node_id, parent_id) VALUES ($1, $2)"
    )
    .bind(branch)
    .bind(node)
    .execute(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    // 4️⃣ Commit transaction
    tx.commit().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    Ok(())
}

pub async fn delete_node(
    node: i32,
    db: &Pool<Postgres>
) -> Result<(), NodeOperationError> {
    let mut tx = db.begin().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    // Check if node exists
    let exists: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM nodes WHERE id = $1"
    )
    .bind(node)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    if exists.is_none() {
        return Err(NodeOperationError::NonexistentNode);
    }

    // Check if the node has remaining parents
    let remaining_parents: Option<i32> = sqlx::query_scalar(
        "SELECT parent_id FROM node_parents WHERE node_id = $1 LIMIT 1"
    )
    .bind(node)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;

    // If the node is still linked to another parent, just unlink it
    if remaining_parents.is_some() {
        sqlx::query("DELETE FROM node_parents WHERE node_id = $1")
            .bind(node)
            .execute(&mut *tx)
            .await
            .map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    } else {
        // Node is fully orphaned, proceed with cascading deletion
        delete_node_bfs(node, &mut tx).await?;
    }

    tx.commit().await.map_err(|_| NodeOperationError::DatabaseOperationFailed)?;
    Ok(())
}
