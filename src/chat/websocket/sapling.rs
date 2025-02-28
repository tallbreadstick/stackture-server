use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use ollama_rs::generation::chat::{
    ChatMessage,
    request::ChatMessageRequest
};
use ollama_rs::{Ollama, coordinator::Coordinator};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct ChatResponse {
    status: String,
    message: String
}


/// Generates the tree from json format.
///
/// * nodes - The nodes of the tree with attributes (id: Int starting from 1, name: String name of the node, description: String description of the node, parents: list of integers with the ids of the parents, children: list of integers with the ids of the children).
#[ollama_rs_macros::function]
async fn generate_tree(nodes: String) -> Result<String, Box<dyn std::error::Error + Sync + Send>> {
    Ok("42.7".to_string())
}


pub async fn sapling_chat(mut socket: WebSocket) {
    let mut ollama = Ollama::new("http://192.168.1.10".to_string(), 11434);
    let mut history: Vec<ChatMessage> = vec![];
    let mut response: ChatResponse = ChatResponse {
        status: String::from("success"),
        message: String::from("")
    };

    // query for chat history

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let res = ollama.send_chat_messages_with_history(
                &mut history, 
                ChatMessageRequest::new(
                    "llama3.1".to_string(),
                    vec![ChatMessage::user(text.to_string())],
                ),
            ).await;

            match res {
                Ok(ai_response) => {
                    // save response to db

                    response.status = "success".to_string();
                    response.message = ai_response.message.content;
                }
                Err(_e) => {
                    response.status = "error".to_string();
                    response.message = "AI Generation Error!".to_string();
                }
            }
            let _ = socket.send(Message::text(serde_json::to_string(&response).unwrap())).await;
        }
    }   
}