use axum::{extract::State, http::StatusCode, response::{IntoResponse, Response}, Json};
use axum_extra::{headers::{authorization::Bearer, Authorization}, TypedHeader};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use super::{api::extract_token_data, atomic::{add_node, borrow_node, create_node, delete_node, drop_node, take_node}};

#[derive(Serialize, Deserialize)]
pub struct CreateRequest {
    workspace_id: i32,
    name: String,
    summary: String
}

#[derive(Serialize)]
pub struct CreateResponse {
    node_id: i32
}

#[derive(Serialize, Deserialize)]
pub struct AddRequest {
    workspace_id: i32,
    node_id: i32,
    name: String,
    summary: String
}

#[derive(Serialize)]
pub struct AddResponse {
    node_id: i32
}

#[derive(Serialize, Deserialize)]
pub struct BorrowRequest {
    node_id: i32,
    branch_id: i32
}

#[derive(Serialize)]
pub struct BorrowResponse;

#[derive(Serialize, Deserialize)]
pub struct DropRequest {
    node_id: i32,
    branch_id: i32
}

#[derive(Serialize)]
pub struct DropResponse;

#[derive(Serialize, Deserialize)]
pub struct TakeRequest {
    node_id: i32,
    branch_id: i32
}

#[derive(Serialize)]
pub struct TakeResponse;

#[derive(Serialize, Deserialize)]
pub struct DeleteRequest {
    node_id: i32
}

#[derive(Serialize)]
pub struct DeleteResponse;

pub async fn create(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateRequest>
) -> Result<Json<CreateResponse>, Response> {
    extract_token_data(auth).map_err(IntoResponse::into_response)?;
    let node_id = create_node(
        payload.workspace_id,
        &payload.name,
        &payload.summary,
        &db
    )
    .await
    .map_err(IntoResponse::into_response)?;
    Ok(Json(CreateResponse { node_id }))
}

pub async fn add(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<AddRequest>
) -> Result<Json<AddResponse>, Response> {
    extract_token_data(auth).map_err(IntoResponse::into_response)?;
    let node_id = add_node(
        payload.workspace_id,
        payload.node_id,
        &payload.name,
        &payload.summary,
        &db
    )
    .await
    .map_err(IntoResponse::into_response)?;
    Ok(Json(AddResponse { node_id }))
}

pub async fn borrow(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<BorrowRequest>
) -> Result<StatusCode, Response> {
    extract_token_data(auth).map_err(IntoResponse::into_response)?;
    borrow_node(
        payload.node_id,
        payload.branch_id,
        &db
    )
    .await
    .map_err(IntoResponse::into_response)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn drop(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<DropRequest>
) -> Result<StatusCode, Response> {
    extract_token_data(auth).map_err(IntoResponse::into_response)?;
    drop_node(
        payload.node_id,
        payload.branch_id,
        &db
    )
    .await
    .map_err(IntoResponse::into_response)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn take(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<TakeRequest>
) -> Result<StatusCode, Response> {
    extract_token_data(auth).map_err(IntoResponse::into_response)?;
    take_node(
        payload.node_id,
        payload.node_id,
        &db
    )
    .await
    .map_err(IntoResponse::into_response)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<DeleteRequest>
) -> Result<StatusCode, Response> {
    extract_token_data(auth).map_err(IntoResponse::into_response)?;
    delete_node(
        payload.node_id,
        &db
    )
    .await
    .map_err(IntoResponse::into_response)?;
    Ok(StatusCode::NO_CONTENT)
}
