mod i18n;
mod templates;

use std::sync::Arc;
use askama::Template;

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};
use serde::Deserialize;
use tokio::sync::Mutex;

use refstore_core::Database;

use i18n::{translations_for, resolve_lang};

pub struct AppState {
    pub db: Mutex<Database>,
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}

/// Start the web admin server
pub async fn run_server(db: Database) {
    let state = Arc::new(AppState {
        db: Mutex::new(db),
    });

    let app = Router::new()
        .route("/", get(list_papers))
        .route("/paper/{id}", get(paper_detail))
        .route("/paper/{id}/delete", get(delete_paper))
        .route("/paper/{id}/toggle", get(toggle_read))
        .route("/tags", get(list_tags))
        .route("/groups", get(list_groups))
        .route("/search", get(search_papers))
        .route("/api/graph/{id}", get(api_citation_graph))
        .route("/lang/{lang}", get(switch_lang))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("Refstore web admin running at http://{}", addr);
    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app,
    )
    .await
    .unwrap();
}

#[derive(Deserialize)]
struct PageQuery {
    page: Option<u32>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

/// Resolve language from request headers
fn lang_from_headers(headers: &HeaderMap) -> String {
    resolve_lang(headers, None)
}

// ── Language switch ──

async fn switch_lang(
    Path(lang): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let referer = headers.get("referer")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("/");
    let cookie = format!("lang={}; Path=/; Max-Age=31536000", lang);
    (
        axum::response::AppendHeaders([(axum::http::header::SET_COOKIE, cookie)]),
        Redirect::to(referer),
    )
}

// ── Handlers ──

async fn list_papers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PageQuery>,
    headers: HeaderMap,
) -> Html<String> {
    let lang = lang_from_headers(&headers);
    let i18n = translations_for(&lang);
    let db = state.db.lock().await;
    let page = query.page.unwrap_or(1);
    let params = refstore_core::model::ListParams {
        page,
        page_size: 20,
        ..Default::default()
    };
    let result = db.list_papers(&params).unwrap();

    let paper_views: Vec<templates::PaperView> = result
        .papers
        .iter()
        .map(|p| {
            let tags = db.get_paper_tags(&p.id).unwrap_or_default();
            templates::PaperView {
                id: p.id.clone(),
                title: p.title.clone(),
                authors_str: p.authors.join(", "),
                tags_str: tags.join(" "),
                is_read: p.is_read,
            }
        })
        .collect();

    let tmpl = templates::ListTemplate {
        papers: paper_views,
        page: result.page,
        total_pages: result.total_pages,
        total: result.total,
        lang,
        i18n,
    };
    Html(tmpl.render().unwrap())
}

async fn paper_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Html<String> {
    let lang = lang_from_headers(&headers);
    let i18n = translations_for(&lang);
    let db = state.db.lock().await;
    let paper = match db.get_paper(&id) {
        Ok(Some(p)) => p,
        _ => return Html("<h1>Paper not found</h1><a href='/'>Back</a>".to_string()),
    };
    let tags = db.get_paper_tags(&id).unwrap_or_default();
    let notes = db.list_notes(&id).unwrap_or_default();
    let citations_raw = db.list_citations(&id).unwrap_or_default();

    let citations: Vec<templates::CitationView> = citations_raw.iter().map(|c| {
        let is_from = c.from_id == paper.id;
        let (source_id, source_title, target_id, target_title) = if is_from {
            let t_title = db.get_paper(&c.to_id).ok().flatten().map(|p| p.title.clone()).unwrap_or_default();
            (paper.id.clone(), paper.title.clone(), c.to_id.clone(), t_title)
        } else {
            let s_title = db.get_paper(&c.from_id).ok().flatten().map(|p| p.title.clone()).unwrap_or_default();
            (c.from_id.clone(), s_title, paper.id.clone(), paper.title.clone())
        };
        templates::CitationView {
            is_from,
            source_id: source_id[..8].to_string(),
            source_title: truncate(&source_title, 40),
            target_id: target_id[..8].to_string(),
            target_title: truncate(&target_title, 40),
            relation: c.relation.as_str().to_string(),
            direction: if is_from { "→".to_string() } else { "←".to_string() },
        }
    }).collect();

    // Check if paper has a citation graph for vis-network
    let has_graph = db.citation_graph(&id, 1).map(|g| !g.edges.is_empty()).unwrap_or(false);

    let tmpl = templates::DetailTemplate {
        paper,
        tags_str: tags.join(", "),
        notes,
        citations,
        has_graph,
        lang,
        i18n,
    };
    Html(tmpl.render().unwrap())
}

/// API endpoint: return citation graph as JSON for vis-network
async fn api_citation_graph(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> axum::Json<serde_json::Value> {
    let db = state.db.lock().await;
    match db.citation_graph(&id, 3) {
        Ok(graph) => {
            let nodes: Vec<serde_json::Value> = graph.papers.iter().map(|p| {
                serde_json::json!({
                    "id": p.id[..8].to_string(),
                    "label": truncate(&p.title, 40),
                    "title": p.title,
                })
            }).collect();
            let edges: Vec<serde_json::Value> = graph.edges.iter().map(|e| {
                serde_json::json!({
                    "from": e.from_id[..8].to_string(),
                    "to": e.to_id[..8].to_string(),
                    "label": e.relation.as_str(),
                    "arrows": "to",
                })
            }).collect();
            axum::Json(serde_json::json!({"nodes": nodes, "edges": edges}))
        }
        Err(_) => axum::Json(serde_json::json!({"nodes": [], "edges": []})),
    }
}

async fn delete_paper(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Redirect {
    let db = state.db.lock().await;
    let _ = db.delete_paper(&id);
    Redirect::to("/")
}

async fn toggle_read(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Redirect {
    let db = state.db.lock().await;
    if let Some(paper) = db.get_paper(&id).unwrap() {
        let _ = db.update_paper(
            &id,
            refstore_core::model::UpdatePaperParams {
                is_read: Some(!paper.is_read),
                ..Default::default()
            },
        );
    }
    Redirect::to(&format!("/paper/{}", id))
}

async fn list_tags(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Html<String> {
    let lang = lang_from_headers(&headers);
    let i18n = translations_for(&lang);
    let db = state.db.lock().await;
    let tags = db.list_tags().unwrap();
    let tmpl = templates::TagsTemplate { tags, lang, i18n };
    Html(tmpl.render().unwrap())
}

async fn list_groups(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Html<String> {
    let lang = lang_from_headers(&headers);
    let i18n = translations_for(&lang);
    let db = state.db.lock().await;
    let groups = db.list_groups().unwrap();
    let tmpl = templates::GroupsTemplate { groups, lang, i18n };
    Html(tmpl.render().unwrap())
}

async fn search_papers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
    headers: HeaderMap,
) -> axum::response::Response {
    let lang = lang_from_headers(&headers);
    let i18n = translations_for(&lang);
    let q = query.q.unwrap_or_default();
    if q.is_empty() {
        return Redirect::to("/").into_response();
    }

    let db = state.db.lock().await;
    let result = db.search_papers(&q, None, None, None, 1, 50).unwrap();

    let paper_views: Vec<templates::PaperView> = result
        .papers
        .iter()
        .map(|p| {
            let tags = db.get_paper_tags(&p.id).unwrap_or_default();
            templates::PaperView {
                id: p.id.clone(),
                title: p.title.clone(),
                authors_str: p.authors.join(", "),
                tags_str: tags.join(" "),
                is_read: p.is_read,
            }
        })
        .collect();

    let tmpl = templates::ListTemplate {
        papers: paper_views,
        page: 1,
        total_pages: 1,
        total: result.total,
        lang,
        i18n,
    };
    Html(tmpl.render().unwrap()).into_response()
}
