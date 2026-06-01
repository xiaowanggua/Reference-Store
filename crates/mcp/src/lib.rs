use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use refstore_core::Database;

/// Run the MCP server (stdio mode)
pub fn run_mcp(db: Database) {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(json!({}));

        let result = match method {
            "initialize" => handle_initialize(),
            "tools/list" => handle_tools_list(),
            "tools/call" => handle_tools_call(&db, &params),
            _ => json!({"error": {"code": -32601, "message": format!("Unknown method: {}", method)}}),
        };

        let response = if result.get("error").is_some() {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": result.get("error").unwrap().clone(),
            })
        } else {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result,
            })
        };

        let output = serde_json::to_string(&response).unwrap();
        writeln!(stdout, "{}", output).ok();
        stdout.flush().ok();
    }
}

fn handle_initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "refstore",
            "version": "0.1.0"
        }
    })
}

fn handle_tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "paper_add",
                "description": "Add a paper to the library",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": {"type": "string", "description": "Paper title"},
                        "authors": {"type": "string", "description": "Comma-separated authors"},
                        "abstract_text": {"type": "string"},
                        "doi": {"type": "string"},
                        "arxiv_id": {"type": "string"},
                        "url": {"type": "string"},
                        "venue": {"type": "string"},
                        "force": {"type": "boolean", "description": "Skip dedup check"}
                    },
                    "required": ["title"]
                }
            },
            {
                "name": "paper_list",
                "description": "List papers with pagination",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page": {"type": "integer", "default": 1},
                        "page_size": {"type": "integer", "default": 10},
                        "tag": {"type": "string"},
                        "group": {"type": "string"},
                        "status": {"type": "string", "enum": ["read", "unread"]}
                    }
                }
            },
            {
                "name": "paper_get",
                "description": "Get paper details by ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {"id": {"type": "string"}},
                    "required": ["id"]
                }
            },
            {
                "name": "paper_search",
                "description": "Search papers by keywords",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "page": {"type": "integer", "default": 1},
                        "page_size": {"type": "integer", "default": 10}
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "paper_tag",
                "description": "Add or remove a tag from a paper",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "tag": {"type": "string"},
                        "action": {"type": "string", "enum": ["add", "remove"]}
                    },
                    "required": ["id", "tag", "action"]
                }
            },
            {
                "name": "paper_note",
                "description": "Add a note to a paper",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "content": {"type": "string"},
                        "note_type": {"type": "string", "enum": ["summary", "method", "result", "thought", "general"], "default": "general"}
                    },
                    "required": ["id", "content"]
                }
            },
            {
                "name": "paper_cite",
                "description": "Add a citation relation between papers",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "from": {"type": "string", "description": "Source paper ID"},
                        "to": {"type": "string", "description": "Target paper ID"},
                        "relation": {"type": "string", "enum": ["cites", "related", "contrasts", "extends", "improves"], "default": "cites"},
                        "strength": {"type": "integer", "minimum": 1, "maximum": 5, "default": 3}
                    },
                    "required": ["from", "to"]
                }
            },
            {
                "name": "paper_count",
                "description": "Count papers matching filters",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tag": {"type": "string"},
                        "group": {"type": "string"},
                        "status": {"type": "string", "enum": ["read", "unread"]}
                    }
                }
            }
        ]
    })
}

fn handle_tools_call(db: &Database, params: &Value) -> Value {
    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "paper_add" => tool_paper_add(db, &args),
        "paper_list" => tool_paper_list(db, &args),
        "paper_get" => tool_paper_get(db, &args),
        "paper_search" => tool_paper_search(db, &args),
        "paper_tag" => tool_paper_tag(db, &args),
        "paper_note" => tool_paper_note(db, &args),
        "paper_cite" => tool_paper_cite(db, &args),
        "paper_count" => tool_paper_count(db, &args),
        _ => json!({"error": {"code": -32601, "message": format!("Unknown tool: {}", tool_name)}}),
    }
}

fn ok_text(text: String) -> Value {
    json!({
        "content": [{"type": "text", "text": text}]
    })
}

fn tool_paper_add(db: &Database, args: &Value) -> Value {
    let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if title.is_empty() {
        return json!({"error": {"code": -32602, "message": "title is required"}});
    }

    let authors = args.get("authors").and_then(|v| v.as_str()).map(|s| {
        s.split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect::<Vec<_>>()
    });

    let params = refstore_core::model::AddPaperParams {
        title,
        authors,
        abstract_text: args.get("abstract_text").and_then(|v| v.as_str()).map(|s| s.to_string()),
        source_url: args.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
        doi: args.get("doi").and_then(|v| v.as_str()).map(|s| s.to_string()),
        arxiv_id: args.get("arxiv_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
        pdf_path: None,
        publish_date: None,
        venue: args.get("venue").and_then(|v| v.as_str()).map(|s| s.to_string()),
        force: args.get("force").and_then(|v| v.as_bool()).unwrap_or(false),
    };

    match db.add_paper(params) {
        Ok(refstore_core::model::DedupResult::New(p)) => {
            ok_text(format!("Added: {} ({})", p.title, &p.id[..8]))
        }
        Ok(refstore_core::model::DedupResult::Duplicate { existing, matched_by }) => {
            ok_text(format!(
                "Duplicate (matched by {}): {} ({})",
                matched_by, existing.title, &existing.id[..8]
            ))
        }
        Err(e) => json!({"error": {"code": -32603, "message": format!("{}", e)}}),
    }
}

fn tool_paper_list(db: &Database, args: &Value) -> Value {
    let params = refstore_core::model::ListParams {
        page: args.get("page").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
        page_size: args.get("page_size").and_then(|v| v.as_u64()).unwrap_or(10) as u32,
        tag: args.get("tag").and_then(|v| v.as_str()).map(|s| s.to_string()),
        group: args.get("group").and_then(|v| v.as_str()).map(|s| s.to_string()),
        is_read: args.get("status").and_then(|v| v.as_str()).and_then(|s| match s {
            "read" => Some(true),
            "unread" => Some(false),
            _ => None,
        }),
        ..Default::default()
    };

    match db.list_papers(&params) {
        Ok(result) => {
            let mut lines = Vec::new();
            for (i, p) in result.papers.iter().enumerate() {
                let read = if p.is_read { "Y" } else { "N" };
                lines.push(format!("{}. {} [{}] {} - {}", i + 1, &p.id[..8], read, p.title, p.authors.join(", ")));
            }
            lines.push(format!("Page {} of {} ({} total)", result.page, result.total_pages, result.total));
            ok_text(lines.join("\n"))
        }
        Err(e) => json!({"error": {"code": -32603, "message": format!("{}", e)}}),
    }
}

fn tool_paper_get(db: &Database, args: &Value) -> Value {
    let id = match args.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return json!({"error": {"code": -32602, "message": "id is required"}}),
    };

    match db.get_paper(id) {
        Ok(Some(p)) => {
            let mut lines = vec![
                format!("Title:    {}", p.title),
                format!("Authors:  {}", p.authors.join(", ")),
            ];
            if let Some(ref abs) = p.abstract_text { lines.push(format!("Abstract: {}", abs)); }
            if let Some(ref url) = p.source_url { lines.push(format!("URL:      {}", url)); }
            if let Some(ref doi) = p.doi { lines.push(format!("DOI:      {}", doi)); }
            if let Some(ref ax) = p.arxiv_id { lines.push(format!("arXiv:    {}", ax)); }
            lines.push(format!("Read:     {}", if p.is_read { "Yes" } else { "No" }));

            let tags = db.get_paper_tags(id).unwrap_or_default();
            if !tags.is_empty() { lines.push(format!("Tags:     {}", tags.join(", "))); }

            ok_text(lines.join("\n"))
        }
        Ok(None) => ok_text("Paper not found.".to_string()),
        Err(e) => json!({"error": {"code": -32603, "message": format!("{}", e)}}),
    }
}

fn tool_paper_search(db: &Database, args: &Value) -> Value {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q,
        None => return json!({"error": {"code": -32602, "message": "query is required"}}),
    };
    let page = args.get("page").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
    let page_size = args.get("page_size").and_then(|v| v.as_u64()).unwrap_or(10) as u32;

    match db.search_papers(query, None, None, None, page, page_size) {
        Ok(result) => {
            let mut lines = Vec::new();
            for (i, p) in result.papers.iter().enumerate() {
                lines.push(format!("{}. {} - {}", i + 1, &p.id[..8], p.title));
            }
            lines.push(format!("Found {} results (page {} of {})", result.total, result.page, result.total_pages));
            ok_text(lines.join("\n"))
        }
        Err(e) => json!({"error": {"code": -32603, "message": format!("{}", e)}}),
    }
}

fn tool_paper_tag(db: &Database, args: &Value) -> Value {
    let id = match args.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return json!({"error": {"code": -32602, "message": "id is required"}}),
    };
    let tag = match args.get("tag").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return json!({"error": {"code": -32602, "message": "tag is required"}}),
    };
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("add");

    match action {
        "add" => {
            match db.add_paper_tag(id, tag) {
                Ok(()) => ok_text(format!("Tagged {} with '{}'", &id[..8.min(id.len())], tag)),
                Err(e) => json!({"error": {"code": -32603, "message": format!("{}", e)}}),
            }
        }
        "remove" => {
            match db.remove_paper_tag(id, tag) {
                Ok(()) => ok_text(format!("Removed tag '{}' from {}", tag, &id[..8.min(id.len())])),
                Err(e) => json!({"error": {"code": -32603, "message": format!("{}", e)}}),
            }
        }
        _ => json!({"error": {"code": -32602, "message": "action must be 'add' or 'remove'"}}),
    }
}

fn tool_paper_note(db: &Database, args: &Value) -> Value {
    let id = match args.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return json!({"error": {"code": -32602, "message": "id is required"}}),
    };
    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json!({"error": {"code": -32602, "message": "content is required"}}),
    };
    let note_type = args.get("note_type").and_then(|v| v.as_str()).unwrap_or("general");

    match db.add_note(id, content, note_type) {
        Ok(note) => ok_text(format!("Note added ({}): {}", note.note_type.as_str(), &note.id[..8])),
        Err(e) => json!({"error": {"code": -32603, "message": format!("{}", e)}}),
    }
}

fn tool_paper_cite(db: &Database, args: &Value) -> Value {
    let from = match args.get("from").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => return json!({"error": {"code": -32602, "message": "from is required"}}),
    };
    let to = match args.get("to").and_then(|v| v.as_str()) {
        Some(t) => t,
        None => return json!({"error": {"code": -32602, "message": "to is required"}}),
    };
    let relation = args.get("relation").and_then(|v| v.as_str()).unwrap_or("cites");
    let strength = args.get("strength").and_then(|v| v.as_u64()).unwrap_or(3) as u8;

    match db.add_citation(from, to, relation, strength, None) {
        Ok(c) => ok_text(format!("Added: {} --[{}]--> {}", &c.from_id[..8], c.relation.as_str(), &c.to_id[..8])),
        Err(e) => json!({"error": {"code": -32603, "message": format!("{}", e)}}),
    }
}

fn tool_paper_count(db: &Database, args: &Value) -> Value {
    let tag = args.get("tag").and_then(|v| v.as_str());
    let group = args.get("group").and_then(|v| v.as_str());
    let is_read = args.get("status").and_then(|v| v.as_str()).and_then(|s| match s {
        "read" => Some(true),
        "unread" => Some(false),
        _ => None,
    });

    match db.count_papers(tag, group, is_read) {
        Ok(count) => ok_text(count.to_string()),
        Err(e) => json!({"error": {"code": -32603, "message": format!("{}", e)}}),
    }
}
