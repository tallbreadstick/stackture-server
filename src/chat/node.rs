use axum::extract::ws::{Message, WebSocket};
use serde::{Serialize, Deserialize};
use reqwest::{Client, header};
use serde_json::json;
use sqlx::{Pool, Postgres};
use super::db::{fetch_messages, insert_message};

#[derive(Deserialize, Serialize, Clone)]
struct ChatResponse {
    pub finish_reason: String,
    pub message: ChatMessage
}

#[derive(Deserialize, Serialize, Default, Clone)]
struct ToolCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
struct ToolCallInfo {
    pub id: String,
    pub function: ToolCall,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallInfo>>
}

#[derive(Deserialize, Serialize, Default)]
struct ChatWrapper {
    pub choices: Vec<ChatResponse>
}

#[derive(Deserialize, Serialize, Default)]
struct Node {
    pub id: u64,
    pub name: String,
    pub summary: String,
    pub icon: String,
    pub parents: Vec<u64>,
    pub branches: Vec<u64>,
    pub optional: bool,
    pub resolved: bool
}

#[derive(Deserialize, Serialize, Default)]
struct Tree {
    pub tree: Vec<Node>
}

#[derive(Serialize, Deserialize, Default)]
pub struct ChatAIResponse {
    pub status: String,
    pub message: String,
    generated_tree: Option<Vec<Node>>
}


pub async fn node_chat(mut socket: WebSocket, tree_exist: bool, chat_id: i32, db: Pool<Postgres>) {
    let client = Client::new();

    let mut headers = header::HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(header::AUTHORIZATION, "Bearer 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".parse().unwrap());

    let mut history: Vec<ChatMessage> = vec![];

    if tree_exist {
        history.push(ChatMessage {
            content: Some("You are an assistant. You are tasked to understand a problem and narrow it down to what the client already finished. You have already generated a tree, therefore you are no longer allowed to generate another one. Your next task is to assist the user with the tree you have generated before. Your direct responses to the user must always be in natural language.".into()),
            role: "system".into(),
            name: None,
            tool_calls: None
        });
    } else {
        history.push(ChatMessage {
            content: Some("You are an assistant. You are tasked to understand a problem and narrow it down to what the client already finished. Do not give a solution, but you will make a roadmap in a form of trees that broke down the main problems into multiple subproblems. Only use the 'generate_tree' function when explicitly asked to create a tree. DO NOT include JSON in your responses to the user—only use JSON within the 'generate_tree' function. Your direct responses to the user must always be in natural language.".into()),
            role: "system".into(),
            name: None,
            tool_calls: None
        });
    }

    let mut response: ChatAIResponse = ChatAIResponse {
        status: String::from("success"),
        message: String::new(),
        generated_tree: None
    };

    history.extend(fetch_messages(chat_id, db).await.unwrap_or(vec![]));

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let user_chatmessage = ChatMessage {
                content: Some(text.to_string()),
                role: "user".into(),
                name: None,
                tool_calls: None
            };

            history.push(user_chatmessage);

            let res = client.post("https://api.groq.com/openai/v1/chat/completions")
                .headers(headers.clone())
                .json(&json!({
                    "messages": history,
                    "model": "llama-3.3-70b-versatile",
                    "tool_choice": "auto",
                    "tools": [
                        {
                            "type": "function",
                            "function": {
                                "name": "generate_tree",
                                "description": "The tree in json format.",
                                "parameters": {
                                    "type": "object",
                                    "properties": {
                                        "tree": {
                                            "type": "array",
                                            "description": "Array of nodes representing the tree structure.",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "id": {
                                                        "type": "integer",
                                                        "description": "Unique ID of the node starting from 1."
                                                    },
                                                    "name": {
                                                        "type": "string",
                                                        "description": "Name of the node."
                                                    },
                                                    "summary": {
                                                        "type": "string",
                                                        "description": "Description of the node."
                                                    },
                                                    "icon": {
                                                        "type": "string",
                                                        "description": "Unicode emoji to represent the node."
                                                    },
                                                    "parents": {
                                                        "type": "array",
                                                        "items": {
                                                            "type": "integer",
                                                            "description": "IDs of parent nodes."
                                                        }
                                                    },
                                                    "branches": {
                                                        "type": "array",
                                                        "items": {
                                                            "type": "integer",
                                                            "description": "IDs of child nodes."
                                                        }
                                                    },
                                                    "optional": {
                                                        "type": "boolean",
                                                        "description": "Indicates if the node is optional for solving the parent."
                                                    },
                                                    "resolved": {
                                                        "type": "boolean",
                                                        "description": "Indicates if the node is complete. ALWAYS FALSE."
                                                    }
                                                },
                                                "required": ["id", "name", "summary", "icon", "parents", "branches", "optional", "resolved"]
                                            }
                                        }
                                    }
                                },
                                "required": ["tree"]
                            }
                        }
                    ]
                })).send().await;

            match res {
                Ok(response_data) => {
                    let chat_wrapper: ChatWrapper = response_data.json().await.unwrap_or(ChatWrapper::default());
                    let mut message: String = chat_wrapper.choices[0].message.content.clone().unwrap_or(String::new());
                    let tools: Vec<ToolCallInfo> = chat_wrapper.choices[0].clone().message.tool_calls.unwrap_or(vec![]);

                    if !message.is_empty() {
                        response.message = message;
                        response.generated_tree = None;
                    } else if tools.len() > 0 {
                        let _tree: Result<Tree, _> = serde_json::from_str(tools[0].clone().function.arguments.as_str());
                        match _tree {
                            Ok(tree) => {
                                response.message = String::from("Here is the generated tree.");
                                response.generated_tree = Some(tree.tree);

                                let mut system_message: &mut ChatMessage = history.get_mut(0).unwrap();
                                system_message.content = Some("You are an assistant. You are tasked to understand a problem and narrow it down to what the client already finished. You have already generated a tree, therefore you are no longer allowed to generate another one. Your next task is to assist the user with the tree you have generated before. Your direct responses to the user must always be in natural language.".into());
                            }
                            Err(_e) => {
                                response.status = "error".to_string();
                                response.message = "Tree Generation Error!".to_string();
                                return;
                            }
                        }
                    }
                    insert_message(chat_id, user_chatmessage, db);
                    history.push(chat_wrapper.choices[0].message.clone());
                    insert_message(chat_id, chat_wrapper.choices[0].message.clone(), db);
                }
                Err(_e) => {
                    response.status = "error".to_string();
                    response.message = "AI Generation Error!".to_string();
                    return;
                }
            }
            let _ = socket.send(Message::text(serde_json::to_string(&response).unwrap_or(String::new()))).await;
        }
    }   
}