pub mod auth;
pub mod db;
pub mod api;
pub mod chat;

use std::net::SocketAddr;

use auth::{login::login, register::register};
use axum::{
    routing::{get, post},
    Router
};
use db::postgres::connect_db;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    
    println!("Starting Stackture backend server...");

    let db_pool = connect_db().await;

    let auth_handler: Router = Router::new()
        .route("/login", post(login))
        .route("/register", post(register))
        .with_state(db_pool.clone());

    let http_server: Router = Router::new()
        .route("/", get(root))
        // .route("/about", todo!())
        // .route("/dashboard", todo!())
        // .route("/workspace/{id}", todo!())
        .route("/chat", get(chat::websocket::websocket_listener))
        .nest("/auth", auth_handler);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, http_server).await.expect("Failed to start backend server!");

}

async fn root() -> String {
    "Skibidi".into()
}