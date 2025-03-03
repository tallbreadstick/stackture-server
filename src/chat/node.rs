use std::env;

use axum::extract::ws::{Message, WebSocket};
use serde::{Serialize, Deserialize};
use reqwest::{Client, header};
use serde_json::json;
use sqlx::{Pool, Postgres};
use super::db::{fetch_messages, insert_message, insert_tree, fetch_current_tree};

#[derive(Deserialize, Serialize, Clone)]
struct ChatResponse {
    pub finish_reason: String,
    pub message: ChatMessage
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct ToolCallInfo {
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

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct Node {
    pub id: i32,
    pub name: String,
    pub summary: String,
    pub icon: String,
    pub parents: Vec<i32>,
    pub branches: Vec<i32>,
    pub optional: bool,
    pub resolved: bool
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct Tree {
    pub tree: Vec<Node>
}

#[derive(Serialize, Deserialize, Default)]
pub struct ChatAIResponse {
    pub status: String,
    pub message: String,
    pub generated_tree: Option<Vec<Node>>
}


pub async fn node_chat(mut socket: WebSocket, workspace_id: i32, node_id: i32, chat_id: i32, db: Pool<Postgres>) {
    let client = Client::new();
    let api_key = format!("Bearer {}", env::var("GROQ_API").expect("GROQ_API must be defined in .env!"));

    let mut headers = header::HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(header::AUTHORIZATION, api_key.parse().unwrap());

    let mut history: Vec<ChatMessage> = vec![];

    // if tree_exist {
    //     history.push(ChatMessage {
    //         content: Some("You are an assistant. You are tasked to understand a problem and narrow it down to what the client already finished. You have already generated a tree, therefore you are no longer allowed to generate another one. Your next task is to assist the user with the tree you have generated before. Your direct responses to the user must always be in natural language.".into()),
    //         role: "system".into(),
    //         name: None,
    //         tool_calls: None
    //     });
    // } else {

    // history.push(ChatMessage {
    //     content: Some("You are an assistant that can generate tree using 'generate_tree' function. You are tasked to understand a problem and narrow it down to what the client already finished. Do not give a solution, but you will make a roadmap in a form of trees that broke down the main problem into multiple subproblems where one can start from the leaves of the tree. Only use the 'generate_tree' function when told to create a tree. DO NOT include any references of 'generate_tree' function in your responses. Your direct responses to the user must always be in natural language. Your responses must not contain any JSON format unless if in the 'generate_tree' function or when the user explicitly asked for a JSON format.".into()),
    //     role: "system".into(),
    //     name: None,
    //     tool_calls: None
    // });

    history.push(ChatMessage {
        content: Some("You are an assistant. You are tasked to understand a problem and narrow it down to what the client already finished. Do not give a solution, but you will make a roadmap in a form of trees that broke down the main problem into multiple subproblems where one can start from the leaves of the tree. Only use the 'generate_tree' function when told to create a tree. DO NOT include JSON or any references of 'generate_tree' function in your responses to the user—only use JSON within the 'generate_tree' function. Your direct responses to the user must always be in natural language.".into()),
        role: "system".into(),
        name: None,
        tool_calls: None
    });

    let mut response: ChatAIResponse = ChatAIResponse {
        status: String::from("success"),
        message: String::new(),
        generated_tree: None
    };

    history.extend(fetch_messages(chat_id, db.clone()).await.unwrap_or(vec![]));

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let mut with_tree: bool = false;

            if let Ok(nodes) = fetch_current_tree(workspace_id, &db).await {
                if let Ok(nodes_json) = serde_json::to_string(&nodes) {
                    history.push(ChatMessage {
                        content: Some(format!("Current Tree in JSON format: {}\nQuery: {}", nodes_json, text)),
                        role: "user".into(),
                        name: None,
                        tool_calls: None
                    });
                    with_tree = true;
                }
            }
    
            if !with_tree {
                history.push(ChatMessage {
                    content: Some(text.to_string()),
                    role: "user".into(),
                    name: None,
                    tool_calls: None
                });
            }

            let res = client.post("https://api.groq.com/openai/v1/chat/completions")
                .headers(headers.clone())
                .json(&json!({
                    "messages": history,
                    "model": "deepseek-r1-distill-llama-70b",
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
                                            "description": "Array of nodes representing the tree structure. There will only be one root node which will be the main problem.",
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
                    if chat_wrapper.choices.len() == 0 {
                        response.status = "error".to_string();
                        response.message = "Request Timeout".to_string();
                        let _ = socket.send(Message::text(serde_json::to_string(&response).unwrap_or(String::new()))).await;
                        return;
                    }

                    let message: String = chat_wrapper.choices[0].message.content.clone().unwrap_or(String::new());
                    let tools: Vec<ToolCallInfo> = chat_wrapper.choices[0].clone().message.tool_calls.unwrap_or(vec![]);

                    if !message.is_empty() {
                        response.message = message;
                        response.generated_tree = None;
                    } else if tools.len() > 0 {
                        let _tree: Result<Tree, _> = serde_json::from_str(tools[0].clone().function.arguments.as_str());
                        match _tree {
                            Ok(tree) => {
                                let mut tree_nodes = tree.tree;

                                if node_id > 0 {
                                    // sub tree insert
                                } else {
                                    if let Err(_) = insert_tree(workspace_id, &mut tree_nodes, &db).await {
                                        response.status = "error".to_string();
                                        response.message = "Tree Insertion Error!".to_string();
                                        let _ = socket.send(Message::text(serde_json::to_string(&response).unwrap_or(String::new()))).await;
                                        return;
                                    }
                                }

                                response.message = String::from("Here is the generated tree.");
                                response.generated_tree = Some(tree_nodes.clone());

                                // let system_message: &mut ChatMessage = history.get_mut(0).unwrap();
                                // system_message.content = Some("You are an assistant. You are tasked to understand a problem and narrow it down to what the client already finished. You have already generated a tree, therefore you are no longer allowed to generate another one. Your next task is to assist the user with the tree you have generated before. Your direct responses to the user must always be in natural language.".into());
                            }
                            Err(_e) => {
                                response.status = "error".to_string();
                                response.message = "Tree Generation Error!".to_string();
                                let _ = socket.send(Message::text(serde_json::to_string(&response).unwrap_or(String::new()))).await;
                                return;
                            }
                        }
                    }
                    if let Some(old_user) = history.last_mut() {
                        if with_tree {
                            old_user.content = Some(text.to_string());
                        }
                        insert_message(chat_id, old_user, &db).await;
                    }
                    history.push(chat_wrapper.choices[0].message.clone());
                    if let Some(new_message) = history.last() {
                        insert_message(chat_id, new_message, &db).await;
                    }

                    while history.len() > 6 {
                        history.remove(0);
                    }
                }
                Err(_e) => {
                    response.status = "error".to_string();
                    response.message = "AI Generation Error!".to_string();
                    let _ = socket.send(Message::text(serde_json::to_string(&response).unwrap_or(String::new()))).await;
                    return;
                }
            }
            let _ = socket.send(Message::text(serde_json::to_string(&response).unwrap_or(String::new()))).await;
        }
    }   
}