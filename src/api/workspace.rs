use super::api::{extract_token_data, ApiError};
use axum::{extract::{Path, State}, Json};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use dotenvy::dotenv;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Pool, Postgres};
use std::env;

// Tree Entity:
// CREATE TABLE trees (
//     id SERIAL PRIMARY KEY,
//     user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
//     title TEXT NOT NULL,
//     description TEXT,
//     tree_data JSONB NOT NULL,
//     created TIMESTAMPTZ DEFAULT now(),
//     updated TIMESTAMPTZ DEFAULT now()
// );

#[derive(Serialize)]
pub struct WorkspaceNode {
    id: i32,
    name: String,
    summary: Option<String>,
    optional: bool,
    resolved: bool,
    icon: Option<String>,
    branches: Vec<i32>, // List of child node IDs
    parents: Vec<i32>,  // List of parent node IDs
}

#[derive(Serialize)]
pub struct Workspace {
    id: i32,
    title: String,
    description: Option<String>,
    root_id: Option<i32>
}

#[derive(Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    title: String,
    description: Option<String>,
}

#[derive(Serialize)]
pub struct CreateWorkspaceResponse {
    workspace_id: i32,
}

pub async fn create_workspace(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateWorkspaceRequest>,
) -> Result<Json<CreateWorkspaceResponse>, ApiError> {
    let token_data = extract_token_data(auth)?;
    let workspace_id = sqlx::query!(
        "INSERT INTO workspaces (user_id, title, description) VALUES ($1, $2, $3) RETURNING id",
        token_data.user_id,
        payload.title,
        payload.description
    )
    .fetch_one(&db)
    .await
    .map_err(|_| ApiError::DatabaseOperationFailed)?
    .id;
    Ok(Json(CreateWorkspaceResponse { workspace_id }))
}

pub async fn get_workspace(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(root_id): Path<i32>,
) -> Result<Json<Vec<WorkspaceNode>>, ApiError> {
    let token_data = extract_token_data(auth)?;

    // Validate that the user owns the workspace containing the root_id
    let user_owns_workspace = sqlx::query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM workspaces WHERE root_id = $1 AND user_id = $2)",
        root_id,
        token_data.user_id
    )
    .fetch_one(&db)
    .await
    .map_err(|_| ApiError::DatabaseOperationFailed)?
    .unwrap_or(false);
    
    if !user_owns_workspace {
        return Err(ApiError::UnauthorizedAccess);
    }

    // Fetch all nodes in the workspace
    let nodes = sqlx::query!(
        "SELECT id, name, summary, optional, resolved, icon FROM nodes WHERE workspace_id = (SELECT id FROM workspaces WHERE root_id = $1)",
        root_id
    )
    .fetch_all(&db)
    .await
    .map_err(|_| ApiError::DatabaseOperationFailed)?;

    // Fetch parent-child relationships
    let relationships = sqlx::query!(
        "SELECT node_id, parent_id FROM node_parents WHERE node_id IN (SELECT id FROM nodes WHERE workspace_id = (SELECT id FROM workspaces WHERE root_id = $1))",
        root_id
    )
    .fetch_all(&db)
    .await
    .map_err(|_| ApiError::DatabaseOperationFailed)?;

    // Build node map
    let mut node_map = std::collections::HashMap::<i32, WorkspaceNode>::new();
    
    for node in nodes {
        node_map.insert(node.id, WorkspaceNode {
            id: node.id,
            name: node.name,
            summary: node.summary,
            optional: node.optional,
            resolved: node.resolved,
            icon: node.icon,
            branches: vec![],
            parents: vec![],
        });
    }

    // Populate parent and child relationships
    for rel in relationships {
        if let Some(node) = node_map.get_mut(&rel.parent_id) {
            node.branches.push(rel.node_id);
        }
        if let Some(child) = node_map.get_mut(&rel.node_id) {
            child.parents.push(rel.parent_id);
        }
    }

    Ok(Json(node_map.into_values().collect()))
}

pub async fn fetch_workspaces(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Vec<Workspace>>, ApiError> {
    // dotenv().expect("Failed to load environment variables!");
    // let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env!");
    // let token_data = decode::<Value>(
    //     auth.token(),
    //     &DecodingKey::from_secret(secret.as_bytes()),
    //     &Validation::default(),
    // )
    // .map_err(|_| ApiError::InvalidToken)?;
    // let user_id = token_data.claims
    //     .get("sub")
    //     .and_then(|v| v.as_str())
    //     .ok_or(ApiError::InvalidToken)?
    //     .parse::<i32>()
    //     .map_err(|_| ApiError::InvalidToken)?;
    let token_data = extract_token_data(auth)?;
    let workspaces = sqlx::query_as!(
        Workspace,
        "SELECT id, title, description, root_id FROM workspaces WHERE user_id = $1",
        token_data.user_id
    )
    .fetch_all(&db)
    .await
    .map_err(|_| ApiError::DatabaseOperationFailed)?;
    Ok(Json(workspaces))
}
