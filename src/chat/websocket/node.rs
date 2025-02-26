use axum::extract::ws::WebSocket;
use ollama_rs::generation::chat::ChatMessage;
use ollama_rs::generation::chat::request::ChatMessageRequest;

struct ChatResponse {
    status: String,
    message: String
}

pub async fn node_chat(mut socket: WebSocket) {
    let mut history: Vec<ChatMessage> = vec![];

    // query for chat history

    
}