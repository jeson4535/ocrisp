use crate::abstractions::AI;
use crate::db::DBTrait;
use anyhow::Result;
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Write};

#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

fn send_response(response: JsonRpcResponse) -> io::Result<()> {
    let json = serde_json::to_string(&response)?;
    println!("{}", json);
    io::stdout().flush()?;
    Ok(())
}

async fn handle_request<DB: DBTrait>(request: JsonRpcRequest, db: &DB, ai: &AI) -> io::Result<()> {
    let response = match request.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "ocrisp",
                    "version": "0.1.0"
                }
            })),
            error: None,
        },
        "notifications/initialized" => {
            // No response needed for notification
            return Ok(());
        }
        "tools/list" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(json!({
                "tools": [
                    {
                        "name": "search",
                        "description": "Search through documents using semantic vector similarity. Finds the most relevant document chunks based on meaning, not just keyword matching. Returns top-k results ranked by relevance. IMPORTANT: This search works best with expanded, contextual queries rather than single keywords.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": {
                                    "type": "string",
                                    "description": "Search query - should be a well-formed question or descriptive phrase for best results. Examples: Instead of 'plants' use 'information about plants and their care', instead of 'API' use 'how to use the API for authentication', instead of 'error' use 'error handling and troubleshooting steps'. Single keywords work but expanded queries with context perform significantly better. The query will be automatically enhanced with synonyms and related terms before embedding."
                                }
                            },
                            "required": ["text"]
                        }
                    }
                ]
            })),
            error: None,
        },
        "tools/call" => {
            let params = request.params.unwrap_or_default();
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = &params["arguments"];

            match tool_name {
                "search" => {
                    let text = arguments["text"].as_str().unwrap_or("No text provided");
                    let result_str = crate::db::simple_search(db, ai, text, None).await;
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: Some(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": result_str
                                }
                            ]
                        })),
                        error: None,
                    }
                }
                _ => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: None,
                    error: Some(json!({
                        "code": -32601,
                        "message": "Tool not found"
                    })),
                },
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(json!({
                "code": -32601,
                "message": "Method not found"
            })),
        },
    };

    send_response(response)
}

pub async fn run_mcp() -> Result<()> {
    let _handle = Qdrant::run_db(true).ok();
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let db = Some(Qdrant::init(None)?).unwrap();
    let ai = AI::new("http://localhost:11434/api/embed", "embeddinggemma", 768);
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());
    
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => {
                if let Err(e) = handle_request(request, &db, &ai).await {
                    eprintln!("Error handling request: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error parsing JSON: {}", e);
            }
        }
    }

    Ok(())
}
