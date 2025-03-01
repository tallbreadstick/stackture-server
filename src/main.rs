pub mod auth;
pub mod db;
pub mod api;
pub mod chat;

use std::net::SocketAddr;

use api::workspace::{create_workspace, delete_workspace, fetch_workspaces, get_workspace};
use auth::{login::login, register::register};
use axum::{
    http::header,
    routing::{delete, get, post, put},
    Router
};
use chat::websocket::websocket_listener;
use db::postgres::connect_db;
use sqlx::{Pool, Postgres};
use tokio::net::TcpListener;
use api::node;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    
    println!("Starting Stackture backend server...");

    let db_pool = connect_db().await;

    let node_handler: Router<Pool<Postgres>> = Router::new()
        .route("/create", post(node::create))
        .route("/add", post(node::add))
        .route("/borrow", put(node::borrow))
        .route("/drop", put(node::drop))
        .route("/take", put(node::take))
        .route("/delete", delete(node::delete))
        .with_state(db_pool.clone());

    let workspace_handler: Router<Pool<Postgres>> = Router::new()
        .route("/create", post(create_workspace))
        .route("/get/{id}", get(get_workspace))
        .route("/delete/{id}", delete(delete_workspace))
        .route("/fetch", get(fetch_workspaces))
        .with_state(db_pool.clone());

    let api_handler: Router<Pool<Postgres>> = Router::new()
        .nest("/workspace", workspace_handler)
        .nest("/node", node_handler)
        .with_state(db_pool.clone());

    let auth_handler: Router<Pool<Postgres>> = Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .with_state(db_pool.clone());

    let http_server: Router = Router::new()
        .route("/", get(root))
        // .route("/about", todo!())
        // .route("/dashboard", todo!())
        // .route("/workspace", todo!())
        .route("/chat", get(websocket_listener))
        .nest("/auth", auth_handler)
        .nest("/api", api_handler)
        .with_state(db_pool.clone())
	.layer(CorsLayer::new()
	.allow_headers(Any)
    	.expose_headers([header::AUTHORIZATION, header::CONTENT_TYPE]));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, http_server).await.expect("Failed to start backend server!");

}

async fn root() -> String {
    "Skibidi".into()
}
