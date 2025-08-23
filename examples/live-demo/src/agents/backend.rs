//! Backend agent implementation
//! Builds the REST API using Axum framework

use synapsed_intent::IntentContext;
use crate::project::ProjectWorkspace;
use anyhow::Result;
use std::fs;
use tracing::info;

pub async fn execute(workspace: &ProjectWorkspace, _context: &IntentContext) -> Result<()> {
    info!("Backend agent building REST API...");
    
    // Create main.rs with Axum server
    let main_content = r#"use axum::{
    routing::{get, post, put, delete},
    Router, Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Todo {
    id: String,
    title: String,
    description: Option<String>,
    completed: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

type TodoStore = Arc<RwLock<HashMap<String, Todo>>>;

#[tokio::main]
async fn main() {
    let store = Arc::new(RwLock::new(HashMap::new()));
    
    let app = Router::new()
        .route("/health", get(health))
        .route("/todos", get(list_todos).post(create_todo))
        .route("/todos/:id", get(get_todo).put(update_todo).delete(delete_todo))
        .with_state(store);
    
    println!("Server running on http://localhost:3000");
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn health() -> &'static str {
    "OK"
}

async fn list_todos(State(store): State<TodoStore>) -> Json<Vec<Todo>> {
    let todos = store.read().unwrap();
    Json(todos.values().cloned().collect())
}

async fn create_todo(
    State(store): State<TodoStore>,
    Json(payload): Json<CreateTodoRequest>,
) -> (StatusCode, Json<Todo>) {
    let todo = Todo {
        id: Uuid::new_v4().to_string(),
        title: payload.title,
        description: payload.description,
        completed: false,
        created_at: chrono::Utc::now(),
    };
    
    store.write().unwrap().insert(todo.id.clone(), todo.clone());
    (StatusCode::CREATED, Json(todo))
}

async fn get_todo(
    Path(id): Path<String>,
    State(store): State<TodoStore>,
) -> Result<Json<Todo>, StatusCode> {
    store.read().unwrap()
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn update_todo(
    Path(id): Path<String>,
    State(store): State<TodoStore>,
    Json(payload): Json<UpdateTodoRequest>,
) -> Result<Json<Todo>, StatusCode> {
    let mut todos = store.write().unwrap();
    todos.get_mut(&id)
        .map(|todo| {
            if let Some(title) = payload.title {
                todo.title = title;
            }
            if let Some(description) = payload.description {
                todo.description = Some(description);
            }
            if let Some(completed) = payload.completed {
                todo.completed = completed;
            }
            Json(todo.clone())
        })
        .ok_or(StatusCode::NOT_FOUND)
}

async fn delete_todo(
    Path(id): Path<String>,
    State(store): State<TodoStore>,
) -> StatusCode {
    match store.write().unwrap().remove(&id) {
        Some(_) => StatusCode::NO_CONTENT,
        None => StatusCode::NOT_FOUND,
    }
}

#[derive(Deserialize)]
struct CreateTodoRequest {
    title: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct UpdateTodoRequest {
    title: Option<String>,
    description: Option<String>,
    completed: Option<bool>,
}
"#;
    
    let src_dir = workspace.root().join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("main.rs"), main_content)?;
    
    info!("  ✓ Created main.rs with REST API endpoints");
    
    // Create Cargo.toml
    let cargo_content = r#"[package]
name = "todo-api"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
"#;
    
    fs::write(workspace.root().join("Cargo.toml"), cargo_content)?;
    info!("  ✓ Created Cargo.toml with dependencies");
    
    Ok(())
}