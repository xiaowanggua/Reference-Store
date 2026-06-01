mod commands;
mod output;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use refstore_core::Database;

#[derive(Parser)]
#[command(name = "ref", version, about = "Refstore — AI-friendly literature management CLI",
    long_about = "Refstore — AI-friendly literature management CLI\n\
        \n\
        Manage academic papers with tags, notes, groups, and citation graphs.\n\
        Supports grep-like search syntax, multi-format import/export, web admin UI,\n\
        and MCP integration for AI tools.\n\
        \n\
        Run 'ref <command> --help' for more information on a command."
)]
struct Cli {
    /// Database file path
    #[arg(long, global = true, env = "REFSTORE_DB", default_value = "~/.refstore/refstore.db")]
    db: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new paper
    ///
    /// Automatically fetches metadata when --doi or --arxiv is provided.
    /// Falls back to manual entry if fetch fails.
    /// Detects duplicates by arXiv ID > DOI > title. Use --force to skip dedup.
    Add(AddArgs),

    /// Show paper details
    ///
    /// Supports short ID prefix (first 8 characters).
    /// Use --format json for machine-readable output.
    Get {
        /// Paper ID (supports short ID prefix)
        id: String,
        /// Output format: text (default) / json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// List papers with pagination and filters
    ///
    /// Output formats: table (default), compact (ID + title only), json.
    /// Filter by --tag, --group, --status (read/unread).
    /// Sort by created (default), title, or date.
    List(ListArgs),

    /// Count papers matching filters
    ///
    /// Returns a single number. Supports --tag, --group, --status filters.
    /// Useful for checking data volume before listing.
    Count(CountArgs),

    /// Search papers by keywords with grep-like syntax
    ///
    /// Uses SQLite FTS5 full-text search. Supported syntax:
    ///   space-separated words → AND: 'deep learning' = deep AND learning
    ///   "quoted phrase" → exact phrase match
    ///   word* → prefix wildcard
    ///   -word or NOT word → exclusion
    ///   word1 OR word2 → either word
    ///
    /// Examples:
    ///   ref search 'deep learning'
    ///   ref search '"attention mechanism"'
    ///   ref search 'transformer OR attention'
    ///   ref search 'deep -learning'
    ///   ref search 'transform*'
    Search(SearchArgs),

    /// Update paper fields
    ///
    /// Only updates fields that are explicitly provided.
    /// Use --read or --unread to toggle reading status.
    Update {
        /// Paper ID (supports short ID prefix)
        id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New authors (comma-separated)
        #[arg(long)]
        authors: Option<String>,
        /// New abstract text
        #[arg(long)]
        abstract_text: Option<String>,
        /// New source URL
        #[arg(long)]
        url: Option<String>,
        /// New DOI
        #[arg(long)]
        doi: Option<String>,
        /// New arXiv ID
        #[arg(long)]
        arxiv: Option<String>,
        /// New local PDF file path
        #[arg(long)]
        pdf: Option<String>,
        /// New publish date (YYYY-MM-DD)
        #[arg(long)]
        date: Option<String>,
        /// New venue (journal/conference)
        #[arg(long)]
        venue: Option<String>,
        /// Mark as read
        #[arg(long)]
        read: bool,
        /// Mark as unread
        #[arg(long)]
        unread: bool,
    },

    /// Delete a paper
    Delete {
        /// Paper ID (supports short ID prefix)
        id: String,
    },

    /// Manage tags (add / remove / list / delete)
    ///
    /// Tags describe paper attributes (e.g., NLP, Transformer).
    /// Supports hierarchical tags (--parent) and aliases (--alias).
    Tag {
        #[command(subcommand)]
        action: TagAction,
    },

    /// Manage notes (add / list / update / delete / search)
    ///
    /// Markdown notes attached to papers. Types: summary, method, result, thought, general.
    Note {
        #[command(subcommand)]
        action: NoteAction,
    },

    /// Manage groups (create / delete / list / assign / unassign)
    ///
    /// Groups organize papers by personal use-case (e.g., "survey", "project-A").
    /// Different from tags: tags describe paper attributes, groups describe your purpose.
    Group {
        #[command(subcommand)]
        action: GroupAction,
    },

    /// Manage citation relations (add / remove / graph / list)
    ///
    /// Relation types: cites, related, contrasts, extends, improves.
    /// Supports graph visualization with --format mermaid.
    Cite {
        #[command(subcommand)]
        action: CiteAction,
    },

    /// Import papers from file
    ///
    /// Supported formats: BibTeX (.bib), JSON backup (.json).
    /// BibTeX imports entries with dedup detection.
    /// JSON backup restores all data (papers, tags, notes, groups, citations).
    Import {
        /// File path (.bib or .json)
        path: String,
    },

    /// Export papers or citation graph
    ///
    /// Formats: bibtex, markdown, mermaid, json.
    /// Filter by --tag, export a single paper by --id,
    /// or export citation graph by --graph <paper-id>.
    Export(ExportArgs),

    /// Create a full JSON backup of the database
    ///
    /// Exports all papers, tags, notes, groups, and citations.
    /// Restore with: ref import <backup.json>
    Backup {
        /// Output file path
        path: String,
    },

    /// Show statistics overview
    ///
    /// Displays: total/read/unread counts, monthly trend chart, tag cloud, and groups.
    Stats,

    /// Start web admin UI at http://localhost:8080
    ///
    /// Provides a browser-based interface for browsing papers, managing tags/groups,
    /// searching, and viewing citation graphs (vis-network).
    /// Supports Chinese/English toggle via /lang/zh or /lang/en.
    Serve,

    /// Start MCP Server on stdio for AI tool integration
    ///
    /// Provides tools: paper_add, paper_list, paper_get, paper_search,
    /// paper_tag, paper_note, paper_cite, paper_count.
    /// Configure in Claude Code, Cursor, or other MCP-compatible AI tools.
    Mcp,

    /// Install ref binary and register PATH
    ///
    /// Copies binary to ~/.refstore/bin/ and adds PATH entry
    /// to .zshrc, .bashrc, .bash_profile, or .profile.
    Setup,
}

// ── Tag 子命令 ──

#[derive(Subcommand)]
enum TagAction {
    /// Add a tag to a paper (auto-creates tag if not exists)
    ///
    /// Use --parent to set a parent tag for hierarchy (e.g., ref tag add <id> BERT --parent NLP).
    /// Use --alias to add a synonym (e.g., ref tag add <id> LLM --alias "Large Language Model").
    Add {
        /// Paper ID (supports short ID prefix)
        id: String,
        /// Tag name
        name: String,
        /// Parent tag name for hierarchy
        #[arg(long)]
        parent: Option<String>,
        /// Tag alias (synonym)
        #[arg(long)]
        alias: Option<String>,
    },
    /// Remove a tag from a paper
    Remove {
        /// Paper ID (supports short ID prefix)
        id: String,
        /// Tag name
        name: String,
    },
    /// List all tags (tree structure)
    List,
    /// Delete a tag entirely (removes from all papers)
    Delete {
        /// Tag name
        name: String,
    },
}

// ── Note 子命令 ──

#[derive(Subcommand)]
enum NoteAction {
    /// Add a Markdown note to a paper
    ///
    /// Note types: summary (paper summary), method (methodology notes),
    /// result (key results), thought (personal thoughts), general (default).
    Add {
        /// Paper ID (supports short ID prefix)
        id: String,
        /// Note content (Markdown supported)
        #[arg(long)]
        content: String,
        /// Note type: summary / method / result / thought / general
        #[arg(long, default_value = "general")]
        note_type: String,
    },
    /// List all notes for a paper
    List {
        /// Paper ID (supports short ID prefix)
        id: String,
    },
    /// Update a note's content or type
    Update {
        /// Note ID
        id: String,
        /// New content
        #[arg(long)]
        content: Option<String>,
        /// New type: summary / method / result / thought / general
        #[arg(long)]
        note_type: Option<String>,
    },
    /// Delete a note
    Delete {
        /// Note ID
        id: String,
    },
    /// Search notes by keyword (with pagination)
    ///
    /// Searches note content with LIKE matching. Paginated by default (10 per page).
    Search {
        /// Search keyword
        keyword: String,
        /// Page number (starts from 1)
        #[arg(long, default_value = "1")]
        page: u32,
        /// Results per page
        #[arg(long, default_value = "10")]
        page_size: u32,
    },
}

// ── Group 子命令 ──

#[derive(Subcommand)]
enum GroupAction {
    /// Create a new group
    Add {
        /// Group name
        name: String,
        /// Group description
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a group (does not delete papers)
    Delete {
        /// Group name
        name: String,
    },
    /// List all groups with paper counts
    List,
    /// Add a paper to a group (auto-creates group if not exists)
    Assign {
        /// Paper ID (supports short ID prefix)
        id: String,
        /// Group name
        group: String,
    },
    /// Remove a paper from a group
    Unassign {
        /// Paper ID (supports short ID prefix)
        id: String,
        /// Group name
        group: String,
    },
}

// ── Cite 子命令 ──

#[derive(Subcommand)]
enum CiteAction {
    /// Add a citation relation between two papers
    ///
    /// Relations: cites (cites), related (related), contrasts (contrasts),
    /// extends (extends), improves (improves).
    /// Strength ranges from 1 (weak) to 5 (strong), default 3.
    Add {
        /// Source paper ID (the one that cites/references)
        from: String,
        /// Target paper ID (the one being cited/referenced)
        to: String,
        /// Relation type: cites / related / contrasts / extends / improves
        #[arg(long, default_value = "cites")]
        relation: String,
        /// Relation strength (1=weak to 5=strong)
        #[arg(long, default_value = "3")]
        strength: u8,
        /// Optional note about this relation
        #[arg(long)]
        note: Option<String>,
    },
    /// Remove a citation relation
    Remove {
        /// Source paper ID
        from: String,
        /// Target paper ID
        to: String,
        /// Relation type to remove
        #[arg(long, default_value = "cites")]
        relation: String,
    },
    /// Show citation graph for a paper (with configurable depth)
    ///
    /// Use --format mermaid to get Mermaid diagram output.
    Graph {
        /// Paper ID (supports short ID prefix)
        id: String,
        /// Graph traversal depth (default: 2)
        #[arg(long, default_value = "2")]
        depth: u32,
        /// Output format: text (default) / mermaid
        #[arg(long, default_value = "text")]
        format: String,
    },
    /// List all direct citations for a paper
    List {
        /// Paper ID (supports short ID prefix)
        id: String,
    },
}

// ── Export 参数 ──

#[derive(clap::Args)]
pub struct ExportArgs {
    /// Export format: bibtex / markdown / mermaid / json
    #[arg(long, default_value = "bibtex")]
    format: String,
    /// Export a specific paper by ID
    #[arg(long)]
    id: Option<String>,
    /// Filter exported papers by tag
    #[arg(long)]
    tag: Option<String>,
    /// Export citation graph for a paper (use with --format mermaid)
    #[arg(long)]
    graph: Option<String>,
}

// ── 参数结构体 ──

#[derive(clap::Args)]
#[command(about = "Add a new paper",
    long_about = "Add a new paper to the library.\n\
        \n\
        When --doi is provided, automatically fetches metadata from CrossRef API.\n\
        When --arxiv is provided, automatically fetches metadata from arXiv API.\n\
        Any explicitly provided flags override auto-fetched values.\n\
        Falls back to manual entry if fetch fails.\n\
        \n\
        Duplicate detection: arXiv ID > DOI > title (case-insensitive).\n\
        Use --force to bypass dedup and always insert.")]
pub struct AddArgs {
    /// Paper title (required for manual entry; auto-filled for DOI/arXiv)
    #[arg(long)]
    title: Option<String>,
    /// Authors (comma-separated, e.g., "Alice,Bob")
    #[arg(long)]
    authors: Option<String>,
    /// Abstract text
    #[arg(long)]
    abstract_text: Option<String>,
    /// Source URL
    #[arg(long)]
    url: Option<String>,
    /// DOI (triggers CrossRef metadata fetch)
    #[arg(long)]
    doi: Option<String>,
    /// arXiv ID (triggers arXiv metadata fetch, e.g., 2301.07041)
    #[arg(long)]
    arxiv: Option<String>,
    /// Local PDF file path
    #[arg(long)]
    pdf: Option<String>,
    /// Publish date (YYYY-MM-DD)
    #[arg(long)]
    date: Option<String>,
    /// Venue (journal or conference name)
    #[arg(long)]
    venue: Option<String>,
    /// Skip dedup check, force insert even if duplicate exists
    #[arg(long)]
    force: bool,
}

#[derive(clap::Args)]
pub struct ListArgs {
    /// Page number (starts from 1)
    #[arg(long, default_value = "1")]
    page: u32,
    /// Results per page
    #[arg(long, default_value = "10")]
    page_size: u32,
    /// Filter by tag name
    #[arg(long)]
    tag: Option<String>,
    /// Filter by group name
    #[arg(long)]
    group: Option<String>,
    /// Filter by read status: read / unread
    #[arg(long)]
    status: Option<String>,
    /// Sort field: created (default) / title / date
    #[arg(long, default_value = "created")]
    sort: String,
    /// Output format: table (default) / compact / json
    #[arg(long, default_value = "table")]
    format: String,
}

#[derive(clap::Args)]
pub struct CountArgs {
    /// Filter by tag name
    #[arg(long)]
    tag: Option<String>,
    /// Filter by group name
    #[arg(long)]
    group: Option<String>,
    /// Filter by read status: read / unread
    #[arg(long)]
    status: Option<String>,
}

#[derive(clap::Args)]
pub struct SearchArgs {
    /// Search query (supports grep-like syntax)
    ///
    /// Syntax: space=AND, "phrase"=exact, word*=prefix,
    /// -word=exclude, word1 OR word2=either
    query: String,
    /// Search scope: all (default) / title / abstract
    #[arg(long, default_value = "all")]
    r#in: String,
    /// Filter results by tag
    #[arg(long)]
    tag: Option<String>,
    /// Filter results by group
    #[arg(long)]
    group: Option<String>,
    /// Page number (starts from 1)
    #[arg(long, default_value = "1")]
    page: u32,
    /// Results per page
    #[arg(long, default_value = "10")]
    page_size: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // setup 不需要数据库
    if matches!(cli.command, Commands::Setup) {
        commands::setup::run()?;
        return Ok(());
    }

    let db_path = expand_tilde(&cli.db);
    let db = Database::open(&db_path)?;

    match cli.command {
        Commands::Add(args) => commands::add::run(&db, &args)?,
        Commands::Get { id, format } => commands::get::run(&db, &id, &format)?,
        Commands::List(args) => commands::list::run(&db, &args)?,
        Commands::Count(args) => {
            let is_read = args.status.as_deref().and_then(|s| match s {
                "read" => Some(true),
                "unread" => Some(false),
                _ => None,
            });
            let count = db.count_papers(args.tag.as_deref(), args.group.as_deref(), is_read)?;
            println!("{}", count);
        }
        Commands::Search(args) => commands::search::run(&db, &args)?,
        Commands::Update {
            id, title, authors, abstract_text, url, doi, arxiv, pdf, date, venue, read, unread,
        } => {
            let is_read = if read { Some(true) } else if unread { Some(false) } else { None };
            let publish_date = date.as_deref().map(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d")).transpose()?;
            let authors_vec = authors.as_deref().map(|s| {
                s.split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect::<Vec<_>>()
            });
            let params = refstore_core::model::UpdatePaperParams {
                title, authors: authors_vec, abstract_text, source_url: url, doi,
                arxiv_id: arxiv, pdf_path: pdf, publish_date, venue, is_read,
            };
            let paper = db.update_paper(&id, params)?;
            println!("Updated: {} ({})", paper.title, &paper.id[..8]);
        }
        Commands::Delete { id } => commands::delete::run(&db, &id)?,
        Commands::Tag { action } => commands::tag::run(&db, &action)?,
        Commands::Note { action } => commands::note::run(&db, &action)?,
        Commands::Group { action } => commands::group::run(&db, &action)?,
        Commands::Cite { action } => commands::cite::run(&db, &action)?,
        Commands::Import { path } => commands::import::run(&db, &path)?,
        Commands::Export(args) => commands::export::run(&db, &args)?,
        Commands::Backup { path } => commands::backup::run(&db, &path)?,
        Commands::Stats => commands::stats::run(&db)?,
        Commands::Serve => {
            refstore_server::run_server(db).await;
        }
        Commands::Mcp => {
            // MCP runs synchronously on stdio
            refstore_mcp::run_mcp(db);
        }
        Commands::Setup => unreachable!(),
    }

    Ok(())
}

/// Expand `~` to the user's home directory
fn expand_tilde(path: &std::path::Path) -> PathBuf {
    if path.starts_with("~") {
        if let Ok(home) = std::env::var("HOME") {
            let home = PathBuf::from(home);
            let rest = path.strip_prefix("~").unwrap_or(path);
            let rest = rest.strip_prefix("/").unwrap_or(rest);
            return home.join(rest);
        }
    }
    path.to_path_buf()
}
