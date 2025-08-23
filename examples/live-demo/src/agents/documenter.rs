//! Documentation agent implementation
//! Generates API documentation

use synapsed_intent::IntentContext;
use crate::project::ProjectWorkspace;
use anyhow::Result;
use std::fs;
use tracing::info;

pub async fn execute(workspace: &ProjectWorkspace, _context: &IntentContext) -> Result<()> {
    info!("Documentation agent generating docs...");
    
    // Create README.md
    let readme_content = r#"# TODO API

A simple REST API for managing TODO items built with Axum.

## Endpoints

### Health Check
- `GET /health` - Returns "OK" if server is running

### TODO Operations

#### List all TODOs
- `GET /todos` - Returns array of all TODO items

#### Create TODO
- `POST /todos` - Creates a new TODO item
  ```json
  {
    "title": "Buy groceries",
    "description": "Milk, eggs, bread"
  }
  ```

#### Get TODO by ID
- `GET /todos/:id` - Returns a specific TODO item

#### Update TODO
- `PUT /todos/:id` - Updates an existing TODO
  ```json
  {
    "title": "Updated title",
    "description": "Updated description",
    "completed": true
  }
  ```

#### Delete TODO
- `DELETE /todos/:id` - Deletes a TODO item

## Running the API

```bash
cargo run
```

The server will start on `http://localhost:3000`

## Testing

```bash
cargo test
```

## Data Model

```json
{
  "id": "uuid",
  "title": "string",
  "description": "string | null",
  "completed": "boolean",
  "created_at": "ISO 8601 timestamp"
}
```
"#;
    
    fs::write(workspace.root().join("README.md"), readme_content)?;
    info!("  ✓ Created README.md");
    
    // Create OpenAPI specification
    let openapi_content = r##"{
  "openapi": "3.0.0",
  "info": {
    "title": "TODO API",
    "version": "1.0.0",
    "description": "A simple REST API for managing TODO items"
  },
  "servers": [
    {
      "url": "http://localhost:3000",
      "description": "Local development server"
    }
  ],
  "paths": {
    "/health": {
      "get": {
        "summary": "Health check",
        "responses": {
          "200": {
            "description": "Server is healthy",
            "content": {
              "text/plain": {
                "schema": {
                  "type": "string",
                  "example": "OK"
                }
              }
            }
          }
        }
      }
    },
    "/todos": {
      "get": {
        "summary": "List all TODOs",
        "responses": {
          "200": {
            "description": "List of TODO items",
            "content": {
              "application/json": {
                "schema": {
                  "type": "array",
                  "items": {
                    "$ref": "#/components/schemas/Todo"
                  }
                }
              }
            }
          }
        }
      },
      "post": {
        "summary": "Create a new TODO",
        "requestBody": {
          "required": true,
          "content": {
            "application/json": {
              "schema": {
                "$ref": "#/components/schemas/CreateTodoRequest"
              }
            }
          }
        },
        "responses": {
          "201": {
            "description": "TODO created successfully",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/Todo"
                }
              }
            }
          }
        }
      }
    }
  },
  "components": {
    "schemas": {
      "Todo": {
        "type": "object",
        "properties": {
          "id": {
            "type": "string",
            "format": "uuid"
          },
          "title": {
            "type": "string"
          },
          "description": {
            "type": "string",
            "nullable": true
          },
          "completed": {
            "type": "boolean"
          },
          "created_at": {
            "type": "string",
            "format": "date-time"
          }
        }
      },
      "CreateTodoRequest": {
        "type": "object",
        "required": ["title"],
        "properties": {
          "title": {
            "type": "string"
          },
          "description": {
            "type": "string",
            "nullable": true
          }
        }
      }
    }
  }
}
"##;
    
    let docs_dir = workspace.root().join("docs");
    fs::create_dir_all(&docs_dir)?;
    fs::write(docs_dir.join("openapi.json"), openapi_content)?;
    info!("  ✓ Created OpenAPI specification");
    
    Ok(())
}