[中文文档](README_zh.md)

# Refstore

A Rust CLI literature management tool designed for AI usage. With AI Skills integration, AI agents can efficiently manage, search, and relate papers in a local library.

## Features

- 📄 **Paper Management** — Manual entry, DOI/arXiv metadata auto-fetch, BibTeX bulk import
- 🔍 **Full-text Search** — FTS5-powered keyword search with grep-like syntax (AND/OR/NOT/phrase/prefix)
- 🏷️ **Tag System** — Hierarchical tags, tag aliases, filter by tag
- 📁 **Group System** — Organize papers by personal use-case (distinct from tags)
- 📝 **Notes** — Markdown notes with multiple types (summary/method/result/thought)
- 🔗 **Citation Graph** — Inter-paper citation relations with Mermaid visualization
- 🤖 **AI Integration** — MCP Server (stdio) + AI Skills, callable by AI tools
- 🌐 **Web Admin** — Lightweight management UI with citation graph visualization (vis-network) and i18n (EN/中文)
- 💾 **Backup & Restore** — Full JSON backup and restore
- ⚡ **Auto Dedup** — Automatic duplicate detection when adding papers (arXiv ID > DOI > title)

## Installation

```bash
# Build
cargo build --release

# Install to ~/.refstore/bin/ and register PATH
./target/release/ref setup
```

## Database Configuration

By default, Refstore uses `~/.refstore/refstore.db` (alongside the installed binary). You can override this:

```bash
# Via --db flag (applies to all commands)
ref --db /path/to/my-library.db list

# Via REFSTORE_DB environment variable
export REFSTORE_DB=/path/to/my-library.db
ref list
```

## Quick Start

```bash
# Add paper by DOI (auto-fetches metadata from CrossRef)
ref add --doi "10.1038/nature12373"

# Add paper by arXiv ID (auto-fetches metadata from arXiv API)
ref add --arxiv 2301.07041

# Add manually with all fields
ref add --title "My Paper" --authors "Alice,Bob" --url "https://..." --venue "NeurIPS 2024"

# Force add even if duplicate detected
ref add --title "Existing Paper" --arxiv 2301.07041 --force

# List all papers (default: table format, 10 per page)
ref list

# List with filters and pagination
ref list --tag NLP --status unread --sort date --page 2 --page-size 20

# Count papers matching filters
ref count --tag "Deep Learning" --status unread

# Search with grep-like syntax
ref search "deep learning"          # AND: both words
ref search '"attention mechanism"'  # Exact phrase
ref search "transformer OR attention"  # OR
ref search "deep -learning"         # Exclude
ref search "transform*"              # Prefix wildcard

# View paper details (supports short ID)
ref get <paper-id>
ref get <paper-id> --format json

# Update paper fields
ref update <paper-id> --title "New Title" --read

# Delete a paper
ref delete <paper-id>

# Tag a paper
ref tag add <paper-id> NLP

# Hierarchical tags and aliases
ref tag add <paper-id> BERT --parent NLP --alias "Bidirectional Encoder"

# Add a note
ref note add <paper-id> --content "Key insight: ..." --note-type summary

# Search notes
ref note search "transformer" --page 1 --page-size 5

# Create citation relation
ref cite add <from-id> <to-id> --relation cites --strength 4 --note "Section 3 extends this"

# View citation graph
ref cite graph <paper-id> --depth 3 --format mermaid

# List all citations for a paper
ref cite list <paper-id>

# Backup entire database to JSON
ref backup papers-backup.json

# Restore from backup (merges into existing database)
ref import papers-backup.json

# Import from BibTeX file
ref import references.bib

# Export papers
ref export --format bibtex
ref export --format markdown --tag "NLP"
ref export --format mermaid --graph <paper-id>

# View statistics (total, read/unread, monthly trend, tag cloud)
ref stats

# Start web admin UI (http://localhost:8080)
ref serve

# Start MCP Server for AI tool integration
ref mcp
```

## Search Syntax

The search command supports grep-like query syntax:

| Query | Meaning |
|-------|---------|
| `deep learning` | AND — matches papers with both words |
| `"attention mechanism"` | Exact phrase match |
| `transform*` | Prefix wildcard |
| `deep -learning` | Exclude — "deep" but NOT "learning" |
| `deep NOT learning` | Exclude (alternative syntax) |
| `transformer OR attention` | OR — either word |
| `BERT` | Single word prefix match (default behavior) |

Search can also be filtered by tag, group, and scoped to title/abstract:

```bash
ref search "transformer" --tag NLP --group survey
ref search "neural" --in title
```

## Command Reference

| Command | Description |
|---------|-------------|
| `ref add` | Add a paper (--doi / --arxiv / manual fields, --force to skip dedup) |
| `ref get <id>` | View paper details (--format text/json, supports short ID) |
| `ref list` | List papers (--tag / --group / --status / --sort / --format / --page) |
| `ref count` | Count matching papers (--tag / --group / --status) |
| `ref search <query>` | Full-text search with grep-like syntax (--in / --tag / --group) |
| `ref update <id>` | Update paper fields (--read / --unread / --title / --doi etc.) |
| `ref delete <id>` | Delete a paper |
| `ref tag add <id> <name>` | Tag a paper (--parent for hierarchy, --alias for synonyms) |
| `ref tag remove <id> <name>` | Remove a tag from a paper |
| `ref tag list` | List all tags (tree structure) |
| `ref tag delete <name>` | Delete a tag entirely |
| `ref note add <id>` | Add a note (--content, --note-type: summary/method/result/thought/general) |
| `ref note list <id>` | List notes for a paper |
| `ref note search <keyword>` | Search notes (--page, --page-size) |
| `ref group add <name>` | Create a group (--description) |
| `ref group assign <id> <group>` | Add paper to group (auto-creates group) |
| `ref group list` | List all groups with paper counts |
| `ref cite add <from> <to>` | Add citation (--relation, --strength 1-5, --note) |
| `ref cite graph <id>` | Citation graph (--depth, --format text/mermaid) |
| `ref import <file>` | Import from .bib or .json backup |
| `ref export` | Export (--format bibtex/markdown/mermaid/json, --tag, --id, --graph) |
| `ref backup <file>` | Full JSON backup |
| `ref stats` | Statistics with monthly trend and tag cloud |
| `ref serve` | Start web admin UI (http://localhost:8080, EN/中文 toggle) |
| `ref mcp` | Start MCP Server (stdio mode) |
| `ref setup` | Install binary and register PATH |

## Output Formats

Three output formats supported for `list` and `search`:

- **table** (default) — Formatted table with columns, human-friendly
- **compact** — Numbered list with ID + title only, minimal output
- **json** — JSON array, for piping/scripting

```bash
ref list --format json
ref list --format compact --page-size 20
```

## Web Admin UI

Start the web admin interface with `ref serve`. It runs at http://localhost:8080 and provides:

- Paper list with pagination and search
- Paper detail view (metadata, notes, citations)
- Read/unread toggle and delete
- Tag and group management pages
- Citation graph visualization (vis-network)
- **Language toggle**: Click "中文" or "EN" in the navigation bar to switch between English and Chinese. Your preference is saved in a cookie.

## Project Structure

```
papermanager/
├── crates/
│   ├── core/          # Core library: models, SQLite storage, search, fetch
│   ├── cli/           # CLI entry point (ref command)
│   ├── server/        # Web admin (Axum + Askama)
│   └── mcp/           # MCP Server (JSON-RPC over stdio)
├── skills/            # AI Skills (generic Markdown instructions)
├── DESIGN.md          # Detailed design document (Chinese)
├── README.md          # This file
└── README_zh.md       # Chinese README
```

## Tech Stack

| Layer | Choice |
|-------|--------|
| Language | Rust (edition 2021) |
| CLI | clap v4 (derive) |
| Database | SQLite (rusqlite) + FTS5 |
| HTTP | axum |
| Templates | askama (compile-time checked) |
| MCP | JSON-RPC over stdio |
| Serialization | serde + serde_json |

## AI Integration

### MCP Server

```bash
# Configure MCP Server in your AI tool
ref mcp
```

Available MCP tools:
- `paper_add` / `paper_list` / `paper_get` / `paper_search`
- `paper_tag` / `paper_note` / `paper_cite` / `paper_count`

### AI Skills

The `skills/` directory contains generic Markdown instruction documents describing which CLI commands to execute for specific user needs.

| Skill | Purpose |
|-------|---------|
| paper-add | Add papers via DOI/arXiv/manual entry, handle duplicates |
| paper-search | Search with grep-like syntax, filter, paginate, count |
| paper-graph | Manage citation relations, visualize graphs |
| paper-notes | Add, search, and manage typed notes |

## License

MIT
