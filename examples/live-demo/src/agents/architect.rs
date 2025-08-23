//! Architect agent that designs the API structure

use crate::project::ProjectWorkspace;
use synapsed_intent::IntentContext;
use anyhow::Result;
use std::fs;
use tracing::info;

/// Execute the architect agent's tasks
pub async fn execute(workspace: &ProjectWorkspace, context: &IntentContext) -> Result<()> {
    info!("    📐 Designing API structure...");
    
    // Create OpenAPI specification
    let openapi_spec = r#"openapi: 3.0.0
info:
  title: TODO API
  version: 1.0.0
  description: A simple TODO list management API

servers:
  - url: http://localhost:3000/api/v1

paths:
  /todos:
    get:
      summary: List all todos
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Todo'
    
    post:
      summary: Create a new todo
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateTodo'
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Todo'
  
  /todos/{id}:
    get:
      summary: Get a todo by ID
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: integer
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Todo'
        '404':
          description: Not found
    
    put:
      summary: Update a todo
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: integer
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/UpdateTodo'
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Todo'
    
    delete:
      summary: Delete a todo
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: integer
      responses:
        '204':
          description: No content

components:
  schemas:
    Todo:
      type: object
      properties:
        id:
          type: integer
        title:
          type: string
        description:
          type: string
        completed:
          type: boolean
        created_at:
          type: string
          format: date-time
        updated_at:
          type: string
          format: date-time
      required:
        - id
        - title
        - completed
        - created_at
    
    CreateTodo:
      type: object
      properties:
        title:
          type: string
        description:
          type: string
      required:
        - title
    
    UpdateTodo:
      type: object
      properties:
        title:
          type: string
        description:
          type: string
        completed:
          type: boolean
"#;
    
    // Write OpenAPI spec
    let spec_path = workspace.root().join("api-design.yaml");
    fs::write(&spec_path, openapi_spec)?;
    info!("    ✓ Created OpenAPI specification: {}", spec_path.display());
    
    // Create database schema
    let schema = r#"-- TODO Application Database Schema

CREATE TABLE IF NOT EXISTS todos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    description TEXT,
    completed BOOLEAN NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for faster queries
CREATE INDEX idx_todos_completed ON todos(completed);
CREATE INDEX idx_todos_created_at ON todos(created_at);

-- Trigger to update updated_at on modification
CREATE TRIGGER update_todos_updated_at
AFTER UPDATE ON todos
BEGIN
    UPDATE todos SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
"#;
    
    let schema_path = workspace.root().join("database-schema.sql");
    fs::write(&schema_path, schema)?;
    info!("    ✓ Created database schema: {}", schema_path.display());
    
    // Create design decisions document
    let decisions = r#"# API Design Decisions

## Architecture Choices

1. **RESTful Design**: Following REST principles for predictable, standard API design
2. **Versioning**: API versioned at URL level (/api/v1) for clear version management
3. **SQLite Database**: Simple, file-based database perfect for TODO application
4. **Axum Framework**: Modern, fast Rust web framework with excellent async support

## Endpoint Design

- **GET /todos**: List all todos with optional filtering
- **POST /todos**: Create new todo with validation
- **GET /todos/{id}**: Retrieve specific todo
- **PUT /todos/{id}**: Full update of todo
- **DELETE /todos/{id}**: Soft or hard delete

## Data Model

- **id**: Auto-incrementing primary key
- **title**: Required field, main todo content
- **description**: Optional detailed description
- **completed**: Boolean status flag
- **created_at**: Automatic timestamp
- **updated_at**: Automatic update tracking

## Error Handling

- Consistent error response format
- Appropriate HTTP status codes
- Detailed error messages for debugging
- Client-friendly error descriptions
"#;
    
    let decisions_path = workspace.root().join("design-decisions.md");
    fs::write(&decisions_path, decisions)?;
    info!("    ✓ Created design decisions: {}", decisions_path.display());
    
    Ok(())
}