//! Standalone Synapsed MCP Server for SDK distribution
//! This is a simplified version that can be compiled independently

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerInfo {
    name: String,
    version: String,
    protocol_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct IntentDeclareParams {
    goal: String,
    description: Option<String>,
    steps: Vec<StepParams>,
    success_criteria: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StepParams {
    name: String,
    action: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct IntentVerifyParams {
    intent_id: String,
    evidence: Value,
}

fn main() {
    // Set up logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    // Get storage path from environment
    let storage_path = env::var("SYNAPSED_STORAGE_PATH")
        .unwrap_or_else(|_| {
            let home = env::var("HOME").or_else(|_| env::var("USERPROFILE")).unwrap_or(".".to_string());
            format!("{}/.synapsed/intents.db", home)
        });
    
    // Ensure storage directory exists
    if let Some(parent) = PathBuf::from(&storage_path).parent() {
        fs::create_dir_all(parent).ok();
    }
    
    log::info!("Synapsed MCP Server starting...");
    log::info!("Storage path: {}", storage_path);
    
    // Initialize SQLite database
    let conn = rusqlite::Connection::open(&storage_path).expect("Failed to open database");
    
    // Create tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS intents (
            id TEXT PRIMARY KEY,
            goal TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            verified INTEGER DEFAULT 0,
            verification_count INTEGER DEFAULT 0
        )",
        [],
    ).expect("Failed to create intents table");
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS verifications (
            id TEXT PRIMARY KEY,
            intent_id TEXT NOT NULL,
            evidence TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            FOREIGN KEY (intent_id) REFERENCES intents(id)
        )",
        [],
    ).expect("Failed to create verifications table");
    
    // MCP stdio transport - read from stdin, write to stdout
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to read line: {}", e);
                continue;
            }
        };
        
        if line.trim().is_empty() {
            continue;
        }
        
        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                log::error!("Failed to parse JSON-RPC request: {}", e);
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: "Parse error".to_string(),
                        data: None,
                    }),
                    id: None,
                };
                writeln!(stdout, "{}", serde_json::to_string(&error_response).unwrap()).ok();
                stdout.flush().ok();
                continue;
            }
        };
        
        log::debug!("Received request: {:?}", request);
        
        // Handle the request
        let response = match request.method.as_str() {
            "initialize" => {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(json!({
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {
                            "name": "synapsed-intent",
                            "version": "0.1.0"
                        },
                        "capabilities": {
                            "tools": {
                                "available": [
                                    {
                                        "name": "intent_declare",
                                        "description": "Declare an intent with goals and verification criteria",
                                        "inputSchema": {
                                            "type": "object",
                                            "properties": {
                                                "goal": {
                                                    "type": "string",
                                                    "description": "The goal of the intent"
                                                },
                                                "description": {
                                                    "type": "string",
                                                    "description": "Detailed description of the intent"
                                                },
                                                "steps": {
                                                    "type": "array",
                                                    "items": {
                                                        "type": "object",
                                                        "properties": {
                                                            "name": {"type": "string"},
                                                            "action": {"type": "string"}
                                                        },
                                                        "required": ["name", "action"]
                                                    }
                                                },
                                                "success_criteria": {
                                                    "type": "array",
                                                    "items": {"type": "string"}
                                                }
                                            },
                                            "required": ["goal", "steps", "success_criteria"]
                                        }
                                    },
                                    {
                                        "name": "intent_verify",
                                        "description": "Verify an intent with evidence",
                                        "inputSchema": {
                                            "type": "object",
                                            "properties": {
                                                "intent_id": {
                                                    "type": "string",
                                                    "description": "ID of the intent to verify"
                                                },
                                                "evidence": {
                                                    "type": "object",
                                                    "description": "Evidence of intent completion"
                                                }
                                            },
                                            "required": ["intent_id", "evidence"]
                                        }
                                    },
                                    {
                                        "name": "intent_status",
                                        "description": "Get status of an intent",
                                        "inputSchema": {
                                            "type": "object",
                                            "properties": {
                                                "intent_id": {
                                                    "type": "string",
                                                    "description": "ID of the intent"
                                                }
                                            },
                                            "required": ["intent_id"]
                                        }
                                    }
                                ]
                            }
                        }
                    })),
                    error: None,
                    id: request.id,
                }
            },
            "tools/call" => {
                let params = request.params.unwrap_or(json!({}));
                let tool_name = params["name"].as_str().unwrap_or("");
                let tool_args = &params["arguments"];
                
                let result = match tool_name {
                    "intent_declare" => {
                        let args: IntentDeclareParams = match serde_json::from_value(tool_args.clone()) {
                            Ok(a) => a,
                            Err(e) => {
                                json!({
                                    "error": format!("Invalid arguments: {}", e)
                                })
                            }
                        };
                        
                        let intent_id = uuid::Uuid::new_v4().to_string();
                        let timestamp = chrono::Utc::now().to_rfc3339();
                        
                        // Store in database
                        match conn.execute(
                            "INSERT INTO intents (id, goal, description, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                            rusqlite::params![
                                &intent_id,
                                &args.goal,
                                &args.description.unwrap_or_default(),
                                "declared",
                                &timestamp
                            ],
                        ) {
                            Ok(_) => {
                                log::info!("Intent declared: {} - {}", intent_id, args.goal);
                                json!({
                                    "intent_id": intent_id,
                                    "status": "declared",
                                    "goal": args.goal,
                                    "steps": args.steps.len(),
                                    "timestamp": timestamp
                                })
                            },
                            Err(e) => {
                                log::error!("Failed to store intent: {}", e);
                                json!({
                                    "error": format!("Failed to store intent: {}", e)
                                })
                            }
                        }
                    },
                    "intent_verify" => {
                        let args: IntentVerifyParams = match serde_json::from_value(tool_args.clone()) {
                            Ok(a) => a,
                            Err(e) => {
                                json!({
                                    "error": format!("Invalid arguments: {}", e)
                                })
                            }
                        };
                        
                        let verification_id = uuid::Uuid::new_v4().to_string();
                        let timestamp = chrono::Utc::now().to_rfc3339();
                        
                        // Store verification
                        match conn.execute(
                            "INSERT INTO verifications (id, intent_id, evidence, timestamp) VALUES (?1, ?2, ?3, ?4)",
                            rusqlite::params![
                                &verification_id,
                                &args.intent_id,
                                &serde_json::to_string(&args.evidence).unwrap(),
                                &timestamp
                            ],
                        ) {
                            Ok(_) => {
                                // Update intent status
                                conn.execute(
                                    "UPDATE intents SET verified = 1, verification_count = verification_count + 1 WHERE id = ?1",
                                    rusqlite::params![&args.intent_id],
                                ).ok();
                                
                                log::info!("Intent verified: {}", args.intent_id);
                                json!({
                                    "verification_id": verification_id,
                                    "intent_id": args.intent_id,
                                    "status": "verified",
                                    "timestamp": timestamp
                                })
                            },
                            Err(e) => {
                                log::error!("Failed to store verification: {}", e);
                                json!({
                                    "error": format!("Failed to store verification: {}", e)
                                })
                            }
                        }
                    },
                    "intent_status" => {
                        let intent_id = tool_args["intent_id"].as_str().unwrap_or("");
                        
                        let mut stmt = conn.prepare(
                            "SELECT goal, status, created_at, verified, verification_count FROM intents WHERE id = ?1"
                        ).unwrap();
                        
                        match stmt.query_row(rusqlite::params![intent_id], |row| {
                            Ok(json!({
                                "intent_id": intent_id,
                                "goal": row.get::<_, String>(0)?,
                                "status": row.get::<_, String>(1)?,
                                "created_at": row.get::<_, String>(2)?,
                                "verified": row.get::<_, i32>(3)? == 1,
                                "verification_count": row.get::<_, i32>(4)?
                            }))
                        }) {
                            Ok(result) => result,
                            Err(_) => json!({
                                "error": "Intent not found"
                            })
                        }
                    },
                    _ => {
                        json!({
                            "error": format!("Unknown tool: {}", tool_name)
                        })
                    }
                };
                
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result).unwrap()
                            }
                        ]
                    })),
                    error: None,
                    id: request.id,
                }
            },
            "tools/list" => {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: Some(json!({
                        "tools": [
                            {
                                "name": "intent_declare",
                                "description": "Declare an intent with goals and verification criteria"
                            },
                            {
                                "name": "intent_verify",
                                "description": "Verify an intent with evidence"
                            },
                            {
                                "name": "intent_status",
                                "description": "Get status of an intent"
                            }
                        ]
                    })),
                    error: None,
                    id: request.id,
                }
            },
            _ => {
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32601,
                        message: format!("Method not found: {}", request.method),
                        data: None,
                    }),
                    id: request.id,
                }
            }
        };
        
        // Send response
        writeln!(stdout, "{}", serde_json::to_string(&response).unwrap()).ok();
        stdout.flush().ok();
    }
    
    log::info!("MCP Server shutting down");
}

// Dependencies for Cargo.toml
// [dependencies]
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// rusqlite = { version = "0.32", features = ["bundled"] }
// uuid = { version = "1.10", features = ["v4"] }
// chrono = "0.4"
// env_logger = "0.11"
// log = "0.4"