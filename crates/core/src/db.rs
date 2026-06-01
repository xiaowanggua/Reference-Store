use std::path::Path;

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::model::*;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("paper not found: {0}")]
    NotFound(String),
}

/// 筛选条件的具体值（可 Clone）
#[derive(Clone, Default)]
struct FilterValues {
    tag: Option<String>,
    group: Option<String>,
    is_read: Option<bool>,
}

impl FilterValues {
    fn from_params(tag: Option<&str>, group: Option<&str>, is_read: Option<bool>) -> Self {
        Self {
            tag: tag.map(|s| s.to_string()),
            group: group.map(|s| s.to_string()),
            is_read,
        }
    }

    /// 构建 WHERE 子句和参数绑定
    fn build_where(&self) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref t) = self.tag {
            let idx = conditions.len() + 1;
            conditions.push(format!(
                "id IN (SELECT pt.paper_id FROM paper_tags pt JOIN tags tg ON pt.tag_id = tg.id WHERE LOWER(tg.name) = LOWER(?{}))",
                idx
            ));
            params.push(Box::new(t.clone()));
        }

        if let Some(ref g) = self.group {
            let idx = conditions.len() + 1;
            conditions.push(format!(
                "id IN (SELECT pg.paper_id FROM paper_groups pg JOIN groups gp ON pg.group_id = gp.id WHERE LOWER(gp.name) = LOWER(?{}))",
                idx
            ));
            params.push(Box::new(g.clone()));
        }

        if let Some(r) = self.is_read {
            let idx = conditions.len() + 1;
            conditions.push(format!("is_read = ?{}", idx));
            params.push(Box::new(r));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        (where_clause, params)
    }
}

/// 搜索查询的词法单元
enum SearchToken {
    Word(String),           // bare word → prefix match: word*
    Phrase(String),         // quoted phrase → exact: "phrase"
    Prefix(String),         // word* → prefix: word*
    Exclude(String),        // -word / NOT word → NOT word*
    ExcludePhrase(String),  // NOT "phrase" → NOT "phrase"
    Or,                     // OR operator
}

/// Convert a SearchToken to its FTS5 query fragment
fn token_to_fts(token: &SearchToken) -> String {
    match token {
        SearchToken::Word(w) => format!("{}*", w),
        SearchToken::Phrase(p) => format!("\"{}\"", p),
        SearchToken::Prefix(p) => p.clone(),
        SearchToken::Exclude(_) | SearchToken::ExcludePhrase(_) | SearchToken::Or => {
            String::new() // handled separately
        }
    }
}

pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    /// 打开数据库，如果文件不存在则自动创建并初始化 schema
    pub fn open(path: &Path) -> Result<Self, DbError> {
        let conn = rusqlite::Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// 在内存中打开数据库（用于测试）
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<(), DbError> {
        self.conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        self.conn.execute_batch(SCHEMA)?;
        Ok(())
    }

    // ── 论文 CRUD ──

    /// 添加论文，自动查重（arxiv_id > doi > title）
    pub fn add_paper(&self, params: AddPaperParams) -> Result<DedupResult, DbError> {
        if !params.force {
            if let Some(dup) = self.check_duplicate(&params)? {
                return Ok(dup);
            }
        }

        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        let authors_json = serde_json::to_string(&params.authors.unwrap_or_default())
            .unwrap_or_else(|_| "[]".to_string());

        self.conn.execute(
            "INSERT INTO papers (id, title, authors, abstract_text, source_url, doi, arxiv_id, pdf_path, publish_date, venue, is_read, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12)",
            rusqlite::params![
                id,
                params.title,
                authors_json,
                params.abstract_text,
                params.source_url,
                params.doi,
                params.arxiv_id,
                params.pdf_path,
                params.publish_date.map(|d| d.to_string()),
                params.venue,
                now,
                now,
            ],
        )?;

        let paper = self.get_paper(&id)?.expect("just inserted");
        Ok(DedupResult::New(paper))
    }

    /// 查重检测：arxiv_id > doi > title
    fn check_duplicate(&self, params: &AddPaperParams) -> Result<Option<DedupResult>, DbError> {
        // 1. arXiv ID 查重（最可靠）
        if let Some(ref arxiv_id) = params.arxiv_id {
            let normalized = normalize_arxiv_id_pub(arxiv_id);
            if let Some(paper) = self.find_by_field("LOWER(arxiv_id)", &normalized.to_lowercase())? {
                return Ok(Some(DedupResult::Duplicate {
                    existing: paper,
                    matched_by: "arxiv_id".to_string(),
                }));
            }
        }

        // 2. DOI 查重
        if let Some(ref doi) = params.doi {
            if let Some(paper) = self.find_by_field("LOWER(doi)", &doi.to_lowercase())? {
                return Ok(Some(DedupResult::Duplicate {
                    existing: paper,
                    matched_by: "doi".to_string(),
                }));
            }
        }

        // 3. 标题查重（精确匹配，忽略大小写）
        if let Some(paper) = self.find_by_field("LOWER(title)", &params.title.to_lowercase())? {
            return Ok(Some(DedupResult::Duplicate {
                existing: paper,
                matched_by: "title".to_string(),
            }));
        }

        Ok(None)
    }

    fn find_by_field(&self, field_expr: &str, value: &str) -> Result<Option<Paper>, DbError> {
        let sql = format!("SELECT * FROM papers WHERE {} = ?1 LIMIT 1", field_expr);
        let mut stmt = self.conn.prepare(&sql)?;
        let mut rows = stmt.query(rusqlite::params![value])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_paper(row)?)),
            None => Ok(None),
        }
    }

    /// 获取单个论文（支持完整 ID 或短 ID 前缀）
    pub fn get_paper(&self, id: &str) -> Result<Option<Paper>, DbError> {
        // 先尝试精确匹配
        let mut stmt = self.conn.prepare("SELECT * FROM papers WHERE id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![id])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(row_to_paper(row)?));
        }
        // 前缀匹配
        let prefix = format!("{}%", id);
        let mut stmt = self.conn.prepare("SELECT * FROM papers WHERE id LIKE ?1 LIMIT 1")?;
        let mut rows = stmt.query(rusqlite::params![prefix])?;
        match rows.next()? {
            Some(row) => Ok(Some(row_to_paper(row)?)),
            None => Ok(None),
        }
    }

    /// 分页列出论文
    pub fn list_papers(&self, params: &ListParams) -> Result<ListResult, DbError> {
        let filter = FilterValues::from_params(
            params.tag.as_deref(),
            params.group.as_deref(),
            params.is_read,
        );

        let order_clause = match params.sort {
            SortField::Created => "ORDER BY created_at DESC",
            SortField::Title => "ORDER BY title ASC",
            SortField::Date => "ORDER BY publish_date DESC NULLS LAST",
        };

        // 总数
        let (where_clause, where_params) = filter.build_where();
        let count_sql = format!("SELECT COUNT(*) FROM papers {}", where_clause);
        let total: i64 = self
            .conn
            .query_row(
                &count_sql,
                rusqlite::params_from_iter(where_params.iter().map(|p| p.as_ref())),
                |row| row.get(0),
            )?;

        let total_pages = if total == 0 {
            0
        } else {
            ((total - 1) / params.page_size as i64 + 1) as u32
        };
        let offset = (params.page.saturating_sub(1)) * params.page_size;

        // 查询（重新构建 params，因为上面的 where_params 已被消费）
        let (where_clause, mut where_params) = filter.build_where();
        where_params.push(Box::new(params.page_size as i64));
        where_params.push(Box::new(offset as i64));

        let query_sql = format!(
            "SELECT * FROM papers {} {} LIMIT ?{} OFFSET ?{}",
            where_clause,
            order_clause,
            where_params.len() - 1,
            where_params.len(),
        );
        let mut stmt = self.conn.prepare(&query_sql)?;

        let papers = stmt
            .query_map(
                rusqlite::params_from_iter(where_params.iter().map(|p| p.as_ref())),
                row_to_paper,
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ListResult {
            papers,
            total,
            page: params.page,
            page_size: params.page_size,
            total_pages,
        })
    }

    /// 统计论文数量（支持筛选）
    pub fn count_papers(
        &self,
        tag: Option<&str>,
        group: Option<&str>,
        is_read: Option<bool>,
    ) -> Result<i64, DbError> {
        let filter = FilterValues::from_params(tag, group, is_read);
        let (where_clause, where_params) = filter.build_where();
        let sql = format!("SELECT COUNT(*) FROM papers {}", where_clause);
        let count: i64 = self.conn.query_row(
            &sql,
            rusqlite::params_from_iter(where_params.iter().map(|p| p.as_ref())),
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// 统计每月新增论文数量（最近12个月）
    pub fn monthly_trend(&self) -> Result<Vec<(String, i64)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT substr(created_at, 1, 7) as month, COUNT(*) as count \
             FROM papers \
             GROUP BY month \
             ORDER BY month DESC \
             LIMIT 12",
        )?;
        let rows: Vec<(String, i64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        // Return in ascending order
        let mut rows = rows;
        rows.reverse();
        Ok(rows)
    }

    /// 解析短 ID 为完整 ID（精确匹配优先，前缀匹配兜底）
    fn resolve_id(&self, id: &str) -> Result<String, DbError> {
        // 精确匹配
        {
            let mut stmt = self.conn.prepare("SELECT id FROM papers WHERE id = ?1")?;
            let mut rows = stmt.query(rusqlite::params![id])?;
            if let Some(row) = rows.next()? {
                return Ok(row.get(0)?);
            }
        }
        // 前缀匹配
        let prefix = format!("{}%", id);
        let mut stmt = self.conn.prepare("SELECT id FROM papers WHERE id LIKE ?1")?;
        let mut rows = stmt.query(rusqlite::params![prefix])?;
        match rows.next()? {
            Some(row) => Ok(row.get(0)?),
            None => Err(DbError::NotFound(id.to_string())),
        }
    }

    /// 更新论文
    pub fn update_paper(&self, id: &str, params: UpdatePaperParams) -> Result<Paper, DbError> {
        let full_id = self.resolve_id(id)?;
        let now = Utc::now().to_rfc3339();
        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now)];

        let mut idx = 2;
        if let Some(ref v) = params.title {
            sets.push(format!("title = ?{}", idx));
            param_values.push(Box::new(v.clone()));
            idx += 1;
        }
        if let Some(ref v) = params.authors {
            let json = serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string());
            sets.push(format!("authors = ?{}", idx));
            param_values.push(Box::new(json));
            idx += 1;
        }
        if let Some(ref v) = params.abstract_text {
            sets.push(format!("abstract_text = ?{}", idx));
            param_values.push(Box::new(v.clone()));
            idx += 1;
        }
        if let Some(ref v) = params.source_url {
            sets.push(format!("source_url = ?{}", idx));
            param_values.push(Box::new(v.clone()));
            idx += 1;
        }
        if let Some(ref v) = params.doi {
            sets.push(format!("doi = ?{}", idx));
            param_values.push(Box::new(v.clone()));
            idx += 1;
        }
        if let Some(ref v) = params.arxiv_id {
            sets.push(format!("arxiv_id = ?{}", idx));
            param_values.push(Box::new(v.clone()));
            idx += 1;
        }
        if let Some(ref v) = params.pdf_path {
            sets.push(format!("pdf_path = ?{}", idx));
            param_values.push(Box::new(v.clone()));
            idx += 1;
        }
        if let Some(ref v) = params.publish_date {
            sets.push(format!("publish_date = ?{}", idx));
            param_values.push(Box::new(v.to_string()));
            idx += 1;
        }
        if let Some(ref v) = params.venue {
            sets.push(format!("venue = ?{}", idx));
            param_values.push(Box::new(v.clone()));
            idx += 1;
        }
        if let Some(v) = params.is_read {
            sets.push(format!("is_read = ?{}", idx));
            param_values.push(Box::new(v));
            idx += 1;
        }

        let sql = format!(
            "UPDATE papers SET {} WHERE id = ?{}",
            sets.join(", "),
            idx
        );
        param_values.push(Box::new(full_id.clone()));

        let changes = self.conn.execute(
            &sql,
            rusqlite::params_from_iter(param_values.iter().map(|p| p.as_ref())),
        )?;

        if changes == 0 {
            return Err(DbError::NotFound(id.to_string()));
        }

        self.get_paper(&full_id)?
            .ok_or(DbError::NotFound(id.to_string()))
    }

    /// 删除论文
    pub fn delete_paper(&self, id: &str) -> Result<bool, DbError> {
        let full_id = match self.resolve_id(id) {
            Ok(fid) => fid,
            Err(DbError::NotFound(_)) => return Ok(false),
            Err(e) => return Err(e),
        };
        let changes =
            self.conn
                .execute("DELETE FROM papers WHERE id = ?1", rusqlite::params![full_id])?;
        Ok(changes > 0)
    }

    // ── 全文搜索 ──

    /// 解析搜索查询为 FTS5 查询语法
    ///
    /// 支持:
    /// - `deep learning` → `deep* AND learning*` (空格 = 隐式 AND)
    /// - `"attention mechanism"` → `"attention mechanism"` (引号 = 精确短语)
    /// - `transform*` → `transform*` (星号 = 前缀)
    /// - `deep -learning` → `deep* NOT learning*` (减号 = 排除)
    /// - `deep NOT learning` → `deep* NOT learning*`
    /// - `transformer OR attention` → `transformer* OR attention*`
    /// - `BERT` → `BERT*` (单词 = 前缀)
    fn parse_search_query(input: &str) -> String {
        let input = input.trim();
        if input.is_empty() {
            return "\"\"".to_string(); // matches nothing
        }

        let mut tokens: Vec<SearchToken> = Vec::new();
        let mut chars = input.char_indices().peekable();

        while let Some(&(i, ch)) = chars.peek() {
            match ch {
                '"' => {
                    // Quoted phrase: read until closing "
                    chars.next(); // consume opening "
                    let start = i + 1;
                    let mut end = start;
                    while let Some(&(j, c)) = chars.peek() {
                        if c == '"' {
                            end = j;
                            chars.next(); // consume closing "
                            break;
                        }
                        chars.next();
                        end = j + c.len_utf8();
                    }
                    let phrase = &input[start..end];
                    if !phrase.is_empty() {
                        tokens.push(SearchToken::Phrase(phrase.replace('"', "\"\"")));
                    }
                }
                ' ' | '\t' => {
                    chars.next(); // skip whitespace
                }
                _ => {
                    // Read a word
                    let start = i;
                    let mut end = start;
                    let is_exclude = ch == '-';

                    while let Some(&(j, c)) = chars.peek() {
                        if c == ' ' || c == '\t' || c == '"' {
                            break;
                        }
                        chars.next();
                        end = j + c.len_utf8();
                    }

                    let word = &input[start..end];

                    if word.eq_ignore_ascii_case("OR") {
                        tokens.push(SearchToken::Or);
                    } else if word.eq_ignore_ascii_case("NOT") {
                        // Peek at next word and mark it as excluded
                        // Skip whitespace
                        while let Some(&(_, c)) = chars.peek() {
                            if c == ' ' || c == '\t' { chars.next(); } else { break; }
                        }
                        // Read next word
                        if let Some(&(j, c)) = chars.peek() {
                            if c == '"' {
                                // NOT "phrase"
                                chars.next();
                                let ps = j + 1;
                                let mut pe = ps;
                                while let Some(&(k, pc)) = chars.peek() {
                                    if pc == '"' { pe = k; chars.next(); break; }
                                    chars.next();
                                    pe = k + pc.len_utf8();
                                }
                                let phrase = &input[ps..pe];
                                if !phrase.is_empty() {
                                    tokens.push(SearchToken::ExcludePhrase(phrase.replace('"', "\"\"")));
                                }
                            } else if c != ' ' && c != '\t' {
                                let ws = j;
                                let mut we = ws;
                                while let Some(&(k, wc)) = chars.peek() {
                                    if wc == ' ' || wc == '\t' || wc == '"' { break; }
                                    chars.next();
                                    we = k + wc.len_utf8();
                                }
                                let w = &input[ws..we];
                                if !w.is_empty() {
                                    tokens.push(SearchToken::Exclude(w.to_string()));
                                }
                            }
                        }
                    } else if is_exclude && word.len() > 1 {
                        // -word (exclude)
                        tokens.push(SearchToken::Exclude(word[1..].to_string()));
                    } else if word.ends_with('*') {
                        tokens.push(SearchToken::Prefix(word.to_string()));
                    } else if !word.is_empty() && word != "-" {
                        tokens.push(SearchToken::Word(word.to_string()));
                    }
                }
            }
        }

        if tokens.is_empty() {
            return "\"\"".to_string();
        }

        // Separate positive tokens and exclusion tokens
        let mut positive: Vec<&SearchToken> = Vec::new();
        let mut exclusions: Vec<&SearchToken> = Vec::new();
        let mut has_or = false;

        for t in &tokens {
            match t {
                SearchToken::Exclude(_) | SearchToken::ExcludePhrase(_) => {
                    exclusions.push(t);
                }
                SearchToken::Or => {
                    has_or = true;
                }
                _ => {
                    positive.push(t);
                }
            }
        }

        // Build positive part
        let positive_part = if positive.is_empty() {
            // Pure exclusion: need a match-all base
            "{papers_fts}".to_string()
        } else if has_or {
            // Collect positive tokens, splitting at OR boundaries
            let mut groups: Vec<String> = Vec::new();
            let mut current: Vec<String> = Vec::new();
            let mut i = 0;
            while i < tokens.len() {
                match &tokens[i] {
                    SearchToken::Or => {
                        if !current.is_empty() {
                            groups.push(current.join(" AND "));
                            current.clear();
                        }
                        // Read next positive token
                        i += 1;
                        while i < tokens.len() {
                            match &tokens[i] {
                                SearchToken::Exclude(_) | SearchToken::ExcludePhrase(_) => { i += 1; continue; }
                                SearchToken::Or => { break; }
                                _ => {
                                    current.push(token_to_fts(&tokens[i]));
                                    break;
                                }
                            }
                        }
                    }
                    SearchToken::Exclude(_) | SearchToken::ExcludePhrase(_) => { /* skip */ }
                    _ => {
                        current.push(token_to_fts(&tokens[i]));
                    }
                }
                i += 1;
            }
            if !current.is_empty() {
                groups.push(current.join(" AND "));
            }

            // Join groups with OR
            let mut result = String::new();
            for (gi, group) in groups.iter().enumerate() {
                if gi > 0 {
                    result.push_str(" OR ");
                }
                result.push_str(group);
            }
            result
        } else {
            // Simple AND chain
            positive.iter().map(|t| token_to_fts(t)).collect::<Vec<_>>().join(" AND ")
        };

        // Append exclusions with NOT
        let mut query = positive_part;
        for ex in &exclusions {
            match ex {
                SearchToken::Exclude(w) => {
                    query.push_str(&format!(" NOT {}*", w));
                }
                SearchToken::ExcludePhrase(p) => {
                    query.push_str(&format!(" NOT \"{}\"", p));
                }
                _ => {}
            }
        }

        query
    }

    /// 全文搜索论文（FTS5），支持 tag/group 过滤
    pub fn search_papers(
        &self,
        query: &str,
        search_in: Option<&str>,
        tag: Option<&str>,
        group: Option<&str>,
        page: u32,
        page_size: u32,
    ) -> Result<ListResult, DbError> {
        let fts_query = Self::parse_search_query(query);
        let _ = search_in; // column-scoped search reserved for future

        // Build the tag/group filter SQL fragments
        let tag_filter = tag.map(|t| {
            format!(
                "p.id IN (SELECT pt.paper_id FROM paper_tags pt JOIN tags tg ON pt.tag_id = tg.id WHERE LOWER(tg.name) = LOWER('{}'))",
                t.replace('\'', "''")
            )
        });
        let group_filter = group.map(|g| {
            format!(
                "p.id IN (SELECT pg.paper_id FROM paper_groups pg JOIN groups gp ON pg.group_id = gp.id WHERE LOWER(gp.name) = LOWER('{}'))",
                g.replace('\'', "''")
            )
        });

        let extra_and: Vec<&str> = [tag_filter.as_deref(), group_filter.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        let extra_sql = if extra_and.is_empty() {
            String::new()
        } else {
            format!(" AND {}", extra_and.join(" AND "))
        };

        // count
        let count_sql = format!(
            "SELECT COUNT(*) FROM papers_fts f JOIN papers p ON p.rowid = f.rowid WHERE papers_fts MATCH ?1{}",
            extra_sql
        );
        let count: i64 = self.conn.query_row(
            &count_sql,
            rusqlite::params![fts_query],
            |row| row.get(0),
        )?;

        let total_pages = if count == 0 { 0 } else { ((count - 1) / page_size as i64 + 1) as u32 };
        let offset = (page.saturating_sub(1)) * page_size;

        // query
        let query_sql = format!(
            "SELECT p.* FROM papers_fts f JOIN papers p ON p.rowid = f.rowid WHERE papers_fts MATCH ?1{} ORDER BY rank LIMIT ?2 OFFSET ?3",
            extra_sql
        );
        let mut stmt = self.conn.prepare(&query_sql)?;
        let papers: Vec<Paper> = stmt
            .query_map(
                rusqlite::params![fts_query, page_size as i64, offset as i64],
                row_to_paper,
            )?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ListResult { papers, total: count, page, page_size, total_pages })
    }

    // ── 标签 CRUD ──

    pub fn create_tag(&self, name: &str, parent_id: Option<&str>, description: Option<&str>, aliases: Option<Vec<String>>) -> Result<Tag, DbError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let aliases_vec: Vec<String> = aliases.unwrap_or_default();
        let aliases_json = serde_json::to_string(&aliases_vec).unwrap_or_else(|_| "[]".to_string());
        self.conn.execute(
            "INSERT INTO tags (id, name, parent_id, aliases, description, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, name, parent_id, aliases_json, description, now],
        )?;
        Ok(Tag {
            id,
            name: name.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            aliases: aliases_vec,
            description: description.map(|s| s.to_string()),
            paper_count: Some(0),
        })
    }

    pub fn list_tags(&self) -> Result<Vec<Tag>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT t.*, COUNT(pt.paper_id) as paper_count FROM tags t LEFT JOIN paper_tags pt ON t.id = pt.tag_id GROUP BY t.id ORDER BY t.name",
        )?;
        let tags = stmt
            .query_map([], |row| {
                Ok(Tag {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    parent_id: row.get("parent_id")?,
                    aliases: serde_json::from_str(&row.get::<_, String>("aliases")?).unwrap_or_default(),
                    description: row.get("description")?,
                    paper_count: Some(row.get::<_, i64>("paper_count")?),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tags)
    }

    pub fn delete_tag(&self, name: &str) -> Result<bool, DbError> {
        let changes = self.conn.execute(
            "DELETE FROM tags WHERE LOWER(name) = LOWER(?1)",
            rusqlite::params![name],
        )?;
        Ok(changes > 0)
    }

    pub fn add_paper_tag(&self, paper_id: &str, tag_name: &str) -> Result<(), DbError> {
        let full_paper_id = self.resolve_id(paper_id)?;
        // 查找或创建标签
        let tag_id = self.get_or_create_tag(tag_name)?;
        self.conn.execute(
            "INSERT OR IGNORE INTO paper_tags (paper_id, tag_id) VALUES (?1, ?2)",
            rusqlite::params![full_paper_id, tag_id],
        )?;
        Ok(())
    }

    pub fn remove_paper_tag(&self, paper_id: &str, tag_name: &str) -> Result<(), DbError> {
        let full_paper_id = self.resolve_id(paper_id)?;
        self.conn.execute(
            "DELETE FROM paper_tags WHERE paper_id = ?1 AND tag_id = (SELECT id FROM tags WHERE LOWER(name) = LOWER(?2))",
            rusqlite::params![full_paper_id, tag_name],
        )?;
        Ok(())
    }

    pub fn get_paper_tags(&self, paper_id: &str) -> Result<Vec<String>, DbError> {
        let full_paper_id = self.resolve_id(paper_id)?;
        let mut stmt = self.conn.prepare(
            "SELECT t.name FROM tags t JOIN paper_tags pt ON t.id = pt.tag_id WHERE pt.paper_id = ?1 ORDER BY t.name",
        )?;
        let tags: Vec<String> = stmt
            .query_map(rusqlite::params![full_paper_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(tags)
    }

    fn get_or_create_tag(&self, name: &str) -> Result<String, DbError> {
        // 查找已有
        let mut stmt = self.conn.prepare("SELECT id FROM tags WHERE LOWER(name) = LOWER(?1)")?;
        let mut rows = stmt.query(rusqlite::params![name])?;
        if let Some(row) = rows.next()? {
            return Ok(row.get(0)?);
        }
        // 创建新的
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO tags (id, name, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, name, now],
        )?;
        Ok(id)
    }

    // ── 笔记 CRUD ──

    pub fn add_note(&self, paper_id: &str, content: &str, note_type: &str) -> Result<Note, DbError> {
        let full_paper_id = self.resolve_id(paper_id)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let nt = NoteType::from_str(note_type).unwrap_or(NoteType::General);
        self.conn.execute(
            "INSERT INTO notes (id, paper_id, content, note_type, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, full_paper_id, content, nt.as_str(), now, now],
        )?;
        Ok(Note {
            id,
            paper_id: full_paper_id,
            content: content.to_string(),
            note_type: nt,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn list_notes(&self, paper_id: &str) -> Result<Vec<Note>, DbError> {
        let full_paper_id = self.resolve_id(paper_id)?;
        let mut stmt = self.conn.prepare(
            "SELECT * FROM notes WHERE paper_id = ?1 ORDER BY created_at DESC",
        )?;
        let notes = stmt
            .query_map(rusqlite::params![full_paper_id], row_to_note)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(notes)
    }

    pub fn update_note(&self, note_id: &str, content: Option<&str>, note_type: Option<&str>) -> Result<Note, DbError> {
        let now = Utc::now().to_rfc3339();
        if let Some(c) = content {
            self.conn.execute(
                "UPDATE notes SET content = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![c, now, note_id],
            )?;
        }
        if let Some(nt) = note_type {
            let nt_str = NoteType::from_str(nt).unwrap_or(NoteType::General).as_str().to_string();
            self.conn.execute(
                "UPDATE notes SET note_type = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![nt_str, now, note_id],
            )?;
        }
        let mut stmt = self.conn.prepare("SELECT * FROM notes WHERE id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![note_id])?;
        match rows.next()? {
            Some(row) => Ok(row_to_note(row)?),
            None => Err(DbError::NotFound(note_id.to_string())),
        }
    }

    pub fn delete_note(&self, note_id: &str) -> Result<bool, DbError> {
        let changes = self.conn.execute(
            "DELETE FROM notes WHERE id = ?1",
            rusqlite::params![note_id],
        )?;
        Ok(changes > 0)
    }

    pub fn search_notes(&self, keyword: &str, page: u32, page_size: u32) -> Result<(Vec<Note>, i64), DbError> {
        let pattern = format!("%{}%", keyword);
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM notes WHERE content LIKE ?1",
            rusqlite::params![pattern.clone()],
            |row| row.get(0),
        )?;
        let offset = (page.saturating_sub(1)) * page_size;
        let mut stmt = self.conn.prepare(
            "SELECT * FROM notes WHERE content LIKE ?1 ORDER BY updated_at DESC LIMIT ?2 OFFSET ?3",
        )?;
        let notes: Vec<Note> = stmt
            .query_map(rusqlite::params![pattern, page_size as i64, offset as i64], row_to_note)?
            .filter_map(|r| r.ok())
            .collect();
        Ok((notes, count))
    }

    // ── 分组 CRUD ──

    pub fn create_group(&self, name: &str, description: Option<&str>) -> Result<Group, DbError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO groups (id, name, description, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, name, description, now],
        )?;
        Ok(Group {
            id,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
        })
    }

    pub fn list_groups(&self) -> Result<Vec<(Group, i64)>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT g.*, COUNT(pg.paper_id) as paper_count FROM groups g LEFT JOIN paper_groups pg ON g.id = pg.group_id GROUP BY g.id ORDER BY g.name",
        )?;
        let groups = stmt
            .query_map([], |row| {
                Ok((
                    Group {
                        id: row.get("id")?,
                        name: row.get("name")?,
                        description: row.get("description")?,
                    },
                    row.get::<_, i64>("paper_count")?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(groups)
    }

    pub fn delete_group(&self, name: &str) -> Result<bool, DbError> {
        let changes = self.conn.execute(
            "DELETE FROM groups WHERE LOWER(name) = LOWER(?1)",
            rusqlite::params![name],
        )?;
        Ok(changes > 0)
    }

    pub fn add_paper_to_group(&self, paper_id: &str, group_name: &str) -> Result<(), DbError> {
        let full_paper_id = self.resolve_id(paper_id)?;
        let group_id = self.get_or_create_group(group_name)?;
        self.conn.execute(
            "INSERT OR IGNORE INTO paper_groups (paper_id, group_id) VALUES (?1, ?2)",
            rusqlite::params![full_paper_id, group_id],
        )?;
        Ok(())
    }

    pub fn remove_paper_from_group(&self, paper_id: &str, group_name: &str) -> Result<(), DbError> {
        let full_paper_id = self.resolve_id(paper_id)?;
        self.conn.execute(
            "DELETE FROM paper_groups WHERE paper_id = ?1 AND group_id = (SELECT id FROM groups WHERE LOWER(name) = LOWER(?2))",
            rusqlite::params![full_paper_id, group_name],
        )?;
        Ok(())
    }

    fn get_or_create_group(&self, name: &str) -> Result<String, DbError> {
        let mut stmt = self.conn.prepare("SELECT id FROM groups WHERE LOWER(name) = LOWER(?1)")?;
        let mut rows = stmt.query(rusqlite::params![name])?;
        if let Some(row) = rows.next()? {
            return Ok(row.get(0)?);
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO groups (id, name, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, name, now],
        )?;
        Ok(id)
    }

    // ── 引用关系 CRUD ──

    pub fn add_citation(
        &self,
        from_id: &str,
        to_id: &str,
        relation: &str,
        strength: u8,
        note: Option<&str>,
    ) -> Result<Citation, DbError> {
        let from = self.resolve_id(from_id)?;
        let to = self.resolve_id(to_id)?;
        let rel = RelationType::from_str(relation).unwrap_or(RelationType::Cites);
        self.conn.execute(
            "INSERT OR REPLACE INTO citations (from_id, to_id, relation, strength, note) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![from, to, rel.as_str(), strength, note],
        )?;
        Ok(Citation {
            from_id: from,
            to_id: to,
            relation: rel,
            strength,
            note: note.map(|s| s.to_string()),
        })
    }

    pub fn remove_citation(&self, from_id: &str, to_id: &str, relation: &str) -> Result<bool, DbError> {
        let from = self.resolve_id(from_id)?;
        let to = self.resolve_id(to_id)?;
        let changes = self.conn.execute(
            "DELETE FROM citations WHERE from_id = ?1 AND to_id = ?2 AND relation = ?3",
            rusqlite::params![from, to, relation],
        )?;
        Ok(changes > 0)
    }

    /// 查询某篇论文的所有直接关系
    pub fn list_citations(&self, paper_id: &str) -> Result<Vec<Citation>, DbError> {
        let full_id = self.resolve_id(paper_id)?;
        let mut stmt = self.conn.prepare(
            "SELECT * FROM citations WHERE from_id = ?1 OR to_id = ?1",
        )?;
        let citations = stmt
            .query_map(rusqlite::params![full_id], row_to_citation)?
            .filter_map(|r| r.ok())
            .collect();
        Ok(citations)
    }

    /// 递归查询关系子图（返回指定深度内所有相关的论文和关系）
    pub fn citation_graph(&self, paper_id: &str, depth: u32) -> Result<GraphResult, DbError> {
        let root = self.resolve_id(paper_id)?;

        // 递归 CTE：从 root 出发，双向扩展
        let sql = format!(
            "WITH RECURSIVE cg AS (
                SELECT from_id, to_id, relation, strength, note, 1 as d
                FROM citations
                WHERE from_id = ?1 OR to_id = ?1
                UNION ALL
                SELECT c.from_id, c.to_id, c.relation, c.strength, c.note, cg.d + 1
                FROM citations c
                JOIN cg ON (c.from_id = cg.to_id OR c.to_id = cg.from_id OR c.from_id = cg.from_id OR c.to_id = cg.to_id)
                WHERE cg.d < ?2 AND c.from_id != c.to_id
            )
            SELECT DISTINCT from_id, to_id, relation, strength, note FROM cg"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let edges: Vec<Citation> = stmt
            .query_map(rusqlite::params![root, depth], row_to_citation)?
            .filter_map(|r| r.ok())
            .collect();

        // 收集所有涉及到的论文 ID
        let mut paper_ids = std::collections::HashSet::new();
        paper_ids.insert(root.clone());
        for e in &edges {
            paper_ids.insert(e.from_id.clone());
            paper_ids.insert(e.to_id.clone());
        }

        // 批量查询这些论文
        let mut papers = Vec::new();
        for pid in &paper_ids {
            if let Some(p) = self.get_paper(pid)? {
                papers.push(p);
            }
        }

        Ok(GraphResult {
            root_id: root,
            papers,
            edges,
        })
    }

    // ── 备份与恢复 ──

    /// Export all data as a JSON-serializable structure
    pub fn export_backup(&self) -> Result<BackupData, DbError> {
        // Papers
        let params = ListParams {
            page_size: 100000,
            ..Default::default()
        };
        let papers = self.list_papers(&params)?.papers;

        // Tags
        let tags = self.list_tags()?;

        // Tag assignments (paper_id, tag_name)
        let mut paper_tags: Vec<(String, String)> = Vec::new();
        for paper in &papers {
            let ptags = self.get_paper_tags(&paper.id)?;
            for tag in &ptags {
                paper_tags.push((paper.id.clone(), tag.clone()));
            }
        }

        // Groups
        let groups_with_count = self.list_groups()?;
        let groups: Vec<Group> = groups_with_count.into_iter().map(|(g, _)| g).collect();

        // Group assignments (paper_id, group_name)
        let mut paper_groups: Vec<(String, String)> = Vec::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT pg.paper_id, g.name FROM paper_groups pg JOIN groups g ON pg.group_id = g.id",
            )?;
            let rows: Vec<(String, String)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .filter_map(|r| r.ok())
                .collect();
            paper_groups = rows;
        }

        // Notes
        let mut notes: Vec<Note> = Vec::new();
        {
            let mut stmt = self.conn.prepare("SELECT * FROM notes ORDER BY created_at")?;
            let rows = stmt.query_map([], row_to_note)?;
            for r in rows {
                if let Ok(note) = r {
                    notes.push(note);
                }
            }
        }

        // Citations
        let mut citations: Vec<Citation> = Vec::new();
        {
            let mut stmt = self.conn.prepare("SELECT * FROM citations")?;
            let rows = stmt.query_map([], row_to_citation)?;
            for r in rows {
                if let Ok(c) = r {
                    citations.push(c);
                }
            }
        }

        Ok(BackupData {
            papers,
            tags,
            paper_tags,
            groups,
            paper_groups,
            notes,
            citations,
        })
    }

    /// Import data from a backup, merging into the current database
    pub fn import_backup(&self, backup: &BackupData) -> Result<BackupStats, DbError> {
        let mut stats = BackupStats::default();

        // Import tags first (papers reference them)
        for tag in &backup.tags {
            // Check if tag already exists
            let exists: bool = {
                let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM tags WHERE LOWER(name) = LOWER(?1)")?;
                let count: i64 = stmt.query_row(rusqlite::params![tag.name], |row| row.get(0))?;
                count > 0
            };
            if !exists {
                let now = Utc::now().to_rfc3339();
                let aliases_json = serde_json::to_string(&tag.aliases).unwrap_or_else(|_| "[]".to_string());
                self.conn.execute(
                    "INSERT INTO tags (id, name, parent_id, aliases, description, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![tag.id, tag.name, tag.parent_id, aliases_json, tag.description, now],
                )?;
                stats.tags_imported += 1;
            } else {
                stats.tags_skipped += 1;
            }
        }

        // Import groups
        for group in &backup.groups {
            let exists: bool = {
                let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM groups WHERE LOWER(name) = LOWER(?1)")?;
                let count: i64 = stmt.query_row(rusqlite::params![group.name], |row| row.get(0))?;
                count > 0
            };
            if !exists {
                let now = Utc::now().to_rfc3339();
                self.conn.execute(
                    "INSERT INTO groups (id, name, description, created_at) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![group.id, group.name, group.description, now],
                )?;
                stats.groups_imported += 1;
            } else {
                stats.groups_skipped += 1;
            }
        }

        // Import papers (use force to skip dedup, use original IDs)
        for paper in &backup.papers {
            // Check if paper already exists (by id)
            let exists: bool = {
                let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM papers WHERE id = ?1")?;
                let count: i64 = stmt.query_row(rusqlite::params![paper.id], |row| row.get(0))?;
                count > 0
            };
            if exists {
                stats.papers_skipped += 1;
                continue;
            }

            let authors_json = serde_json::to_string(&paper.authors).unwrap_or_else(|_| "[]".to_string());
            self.conn.execute(
                "INSERT INTO papers (id, title, authors, abstract_text, source_url, doi, arxiv_id, pdf_path, publish_date, venue, is_read, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                rusqlite::params![
                    paper.id, paper.title, authors_json, paper.abstract_text,
                    paper.source_url, paper.doi, paper.arxiv_id, paper.pdf_path,
                    paper.publish_date, paper.venue, paper.is_read,
                    paper.created_at, paper.updated_at,
                ],
            )?;
            stats.papers_imported += 1;
        }

        // Import tag assignments
        for (paper_id, tag_name) in &backup.paper_tags {
            // Resolve paper_id (might be short or full)
            let resolved = match self.resolve_id(paper_id) {
                Ok(id) => id,
                Err(_) => continue, // paper doesn't exist, skip
            };
            // Resolve tag
            let tag_id: Option<String> = {
                let mut stmt = self.conn.prepare("SELECT id FROM tags WHERE LOWER(name) = LOWER(?1)")?;
                let mut rows = stmt.query(rusqlite::params![tag_name])?;
                rows.next()?.map(|r| r.get(0)).transpose().ok().flatten()
            };
            if let Some(tid) = tag_id {
                self.conn.execute(
                    "INSERT OR IGNORE INTO paper_tags (paper_id, tag_id) VALUES (?1, ?2)",
                    rusqlite::params![resolved, tid],
                )?;
            }
        }

        // Import group assignments
        for (paper_id, group_name) in &backup.paper_groups {
            let resolved = match self.resolve_id(paper_id) {
                Ok(id) => id,
                Err(_) => continue,
            };
            let group_id: Option<String> = {
                let mut stmt = self.conn.prepare("SELECT id FROM groups WHERE LOWER(name) = LOWER(?1)")?;
                let mut rows = stmt.query(rusqlite::params![group_name])?;
                rows.next()?.map(|r| r.get(0)).transpose().ok().flatten()
            };
            if let Some(gid) = group_id {
                self.conn.execute(
                    "INSERT OR IGNORE INTO paper_groups (paper_id, group_id) VALUES (?1, ?2)",
                    rusqlite::params![resolved, gid],
                )?;
            }
        }

        // Import notes
        for note in &backup.notes {
            // Check if note exists
            let exists: bool = {
                let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM notes WHERE id = ?1")?;
                let count: i64 = stmt.query_row(rusqlite::params![note.id], |row| row.get(0))?;
                count > 0
            };
            if exists {
                stats.notes_skipped += 1;
                continue;
            }
            // Check if paper exists
            let paper_exists: bool = {
                let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM papers WHERE id = ?1")?;
                let count: i64 = stmt.query_row(rusqlite::params![note.paper_id], |row| row.get(0))?;
                count > 0
            };
            if !paper_exists {
                stats.notes_skipped += 1;
                continue;
            }
            self.conn.execute(
                "INSERT INTO notes (id, paper_id, content, note_type, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![note.id, note.paper_id, note.content, note.note_type.as_str(), note.created_at, note.updated_at],
            )?;
            stats.notes_imported += 1;
        }

        // Import citations
        for cite in &backup.citations {
            // Check if both papers exist
            let from_exists: bool = {
                let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM papers WHERE id = ?1")?;
                let count: i64 = stmt.query_row(rusqlite::params![cite.from_id], |row| row.get(0))?;
                count > 0
            };
            let to_exists: bool = {
                let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM papers WHERE id = ?1")?;
                let count: i64 = stmt.query_row(rusqlite::params![cite.to_id], |row| row.get(0))?;
                count > 0
            };
            if !from_exists || !to_exists {
                stats.citations_skipped += 1;
                continue;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO citations (from_id, to_id, relation, strength, note) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![cite.from_id, cite.to_id, cite.relation.as_str(), cite.strength, cite.note],
            )?;
            stats.citations_imported += 1;
        }

        Ok(stats)
    }
}

/// 图查询结果
pub struct GraphResult {
    pub root_id: String,
    pub papers: Vec<Paper>,
    pub edges: Vec<Citation>,
}

/// Full backup data for JSON export/import
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BackupData {
    pub papers: Vec<Paper>,
    pub tags: Vec<Tag>,
    pub paper_tags: Vec<(String, String)>,
    pub groups: Vec<Group>,
    pub paper_groups: Vec<(String, String)>,
    pub notes: Vec<Note>,
    pub citations: Vec<Citation>,
}

/// Statistics from a backup import
#[derive(Default)]
pub struct BackupStats {
    pub papers_imported: u32,
    pub papers_skipped: u32,
    pub tags_imported: u32,
    pub tags_skipped: u32,
    pub groups_imported: u32,
    pub groups_skipped: u32,
    pub notes_imported: u32,
    pub notes_skipped: u32,
    pub citations_imported: u32,
    pub citations_skipped: u32,
}

/// 从行数据映射到 Paper 结构体
fn row_to_paper(row: &rusqlite::Row<'_>) -> Result<Paper, rusqlite::Error> {
    let authors_str: String = row.get("authors")?;
    let authors: Vec<String> = serde_json::from_str(&authors_str).unwrap_or_default();

    Ok(Paper {
        id: row.get("id")?,
        title: row.get("title")?,
        authors,
        abstract_text: row.get("abstract_text")?,
        source_url: row.get("source_url")?,
        doi: row.get("doi")?,
        arxiv_id: row.get("arxiv_id")?,
        pdf_path: row.get("pdf_path")?,
        publish_date: row.get("publish_date")?,
        venue: row.get("venue")?,
        is_read: row.get("is_read")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// 从行数据映射到 Note 结构体
fn row_to_note(row: &rusqlite::Row<'_>) -> Result<Note, rusqlite::Error> {
    let note_type_str: String = row.get("note_type")?;
    Ok(Note {
        id: row.get("id")?,
        paper_id: row.get("paper_id")?,
        content: row.get("content")?,
        note_type: NoteType::from_str(&note_type_str).unwrap_or(NoteType::General),
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

/// 从行数据映射到 Citation 结构体
fn row_to_citation(row: &rusqlite::Row<'_>) -> Result<Citation, rusqlite::Error> {
    let rel_str: String = row.get("relation")?;
    Ok(Citation {
        from_id: row.get("from_id")?,
        to_id: row.get("to_id")?,
        relation: RelationType::from_str(&rel_str).unwrap_or(RelationType::Cites),
        strength: row.get("strength")?,
        note: row.get("note")?,
    })
}

/// 标准化 arXiv ID（处理 2301.07041 / arXiv:2301.07041 / https://arxiv.org/abs/2301.07041 等各种格式）
pub fn normalize_arxiv_id_pub(id: &str) -> String {
    let id = id.trim();
    let id = id
        .strip_prefix("https://arxiv.org/abs/")
        .or_else(|| id.strip_prefix("http://arxiv.org/abs/"))
        .unwrap_or(id);
    let id = id.strip_prefix("arXiv:").unwrap_or(id);
    let id = id.split_once('v').map(|(s, _)| s).unwrap_or(id);
    id.trim().to_string()
}

// ── Schema ──

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS papers (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    authors TEXT NOT NULL DEFAULT '[]',
    abstract_text TEXT,
    source_url TEXT,
    doi TEXT,
    arxiv_id TEXT,
    pdf_path TEXT,
    publish_date TEXT,
    venue TEXT,
    is_read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_papers_arxiv_id ON papers(LOWER(arxiv_id));
CREATE INDEX IF NOT EXISTS idx_papers_doi ON papers(LOWER(doi));
CREATE INDEX IF NOT EXISTS idx_papers_title ON papers(LOWER(title));

CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    parent_id TEXT REFERENCES tags(id) ON DELETE SET NULL,
    aliases TEXT NOT NULL DEFAULT '[]',
    description TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS paper_tags (
    paper_id TEXT NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (paper_id, tag_id)
);

CREATE TABLE IF NOT EXISTS groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS paper_groups (
    paper_id TEXT NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    PRIMARY KEY (paper_id, group_id)
);

CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    paper_id TEXT NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    note_type TEXT NOT NULL DEFAULT 'general',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS citations (
    from_id TEXT NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    to_id TEXT NOT NULL REFERENCES papers(id) ON DELETE CASCADE,
    relation TEXT NOT NULL DEFAULT 'cites',
    strength INTEGER NOT NULL DEFAULT 3,
    note TEXT,
    PRIMARY KEY (from_id, to_id, relation)
);

-- FTS5 全文搜索虚拟表
CREATE VIRTUAL TABLE IF NOT EXISTS papers_fts USING fts5(
    title,
    abstract_text,
    content='papers',
    content_rowid='rowid'
);

-- FTS5 同步触发器：INSERT
CREATE TRIGGER IF NOT EXISTS papers_fts_insert AFTER INSERT ON papers BEGIN
    INSERT INTO papers_fts (rowid, title, abstract_text) VALUES (new.rowid, new.title, COALESCE(new.abstract_text, ''));
END;

-- FTS5 同步触发器：DELETE
CREATE TRIGGER IF NOT EXISTS papers_fts_delete AFTER DELETE ON papers BEGIN
    INSERT INTO papers_fts (papers_fts, rowid, title, abstract_text) VALUES ('delete', old.rowid, old.title, COALESCE(old.abstract_text, ''));
END;

-- FTS5 同步触发器：UPDATE
CREATE TRIGGER IF NOT EXISTS papers_fts_update AFTER UPDATE ON papers BEGIN
    INSERT INTO papers_fts (papers_fts, rowid, title, abstract_text) VALUES ('delete', old.rowid, old.title, COALESCE(old.abstract_text, ''));
    INSERT INTO papers_fts (rowid, title, abstract_text) VALUES (new.rowid, new.title, COALESCE(new.abstract_text, ''));
END;
";

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().unwrap()
    }

    fn add_sample_paper(db: &Database, title: &str, arxiv: Option<&str>) -> Paper {
        let result = db
            .add_paper(AddPaperParams {
                title: title.to_string(),
                authors: Some(vec!["Author One".to_string(), "Author Two".to_string()]),
                abstract_text: Some(format!("Abstract for {}", title)),
                source_url: None,
                doi: None,
                arxiv_id: arxiv.map(|s| s.to_string()),
                pdf_path: None,
                publish_date: None,
                venue: Some("NeurIPS".to_string()),
                force: false,
            })
            .unwrap();
        match result {
            DedupResult::New(p) => p,
            DedupResult::Duplicate { existing, .. } => existing,
        }
    }

    // ── Phase 1: Paper CRUD ──

    #[test]
    fn test_add_paper() {
        let db = test_db();
        let paper = add_sample_paper(&db, "Test Paper", None);
        assert!(!paper.id.is_empty());
        assert_eq!(paper.title, "Test Paper");
        assert_eq!(paper.authors.len(), 2);
        assert!(!paper.is_read);
    }

    #[test]
    fn test_get_paper_full_id() {
        let db = test_db();
        let paper = add_sample_paper(&db, "Get Test", None);
        let got = db.get_paper(&paper.id).unwrap().unwrap();
        assert_eq!(got.id, paper.id);
        assert_eq!(got.title, "Get Test");
    }

    #[test]
    fn test_get_paper_short_id() {
        let db = test_db();
        let paper = add_sample_paper(&db, "Short ID Test", None);
        let short = &paper.id[..8];
        let got = db.get_paper(short).unwrap().unwrap();
        assert_eq!(got.id, paper.id);
    }

    #[test]
    fn test_get_paper_not_found() {
        let db = test_db();
        assert!(db.get_paper("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_update_paper() {
        let db = test_db();
        let paper = add_sample_paper(&db, "Before Update", None);
        let updated = db
            .update_paper(
                &paper.id,
                UpdatePaperParams {
                    title: Some("After Update".to_string()),
                    is_read: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(updated.title, "After Update");
        assert!(updated.is_read);
    }

    // Need Default for UpdatePaperParams
    // Actually, let me just construct it manually in tests

    #[test]
    fn test_delete_paper() {
        let db = test_db();
        let paper = add_sample_paper(&db, "To Delete", None);
        assert!(db.delete_paper(&paper.id).unwrap());
        assert!(db.get_paper(&paper.id).unwrap().is_none());
    }

    #[test]
    fn test_delete_paper_not_found() {
        let db = test_db();
        assert!(!db.delete_paper("nonexistent").unwrap());
    }

    #[test]
    fn test_list_papers_pagination() {
        let db = test_db();
        for i in 0..15 {
            add_sample_paper(&db, &format!("Paper {}", i), None);
        }
        let result = db
            .list_papers(&ListParams {
                page: 1,
                page_size: 10,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(result.papers.len(), 10);
        assert_eq!(result.total, 15);
        assert_eq!(result.total_pages, 2);

        let page2 = db
            .list_papers(&ListParams {
                page: 2,
                page_size: 10,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(page2.papers.len(), 5);
    }

    #[test]
    fn test_count_papers() {
        let db = test_db();
        add_sample_paper(&db, "A", None);
        add_sample_paper(&db, "B", None);
        assert_eq!(db.count_papers(None, None, None).unwrap(), 2);
    }

    // ── Phase 1: Dedup ──

    #[test]
    fn test_dedup_by_arxiv_id() {
        let db = test_db();
        add_sample_paper(&db, "Paper A", Some("2301.07041"));
        let result = db
            .add_paper(AddPaperParams {
                title: "Paper B".to_string(),
                arxiv_id: Some("2301.07041".to_string()),
                force: false,
                ..Default::default()
            })
            .unwrap();
        match result {
            DedupResult::Duplicate { matched_by, .. } => {
                assert_eq!(matched_by, "arxiv_id");
            }
            DedupResult::New(_) => panic!("Should be duplicate"),
        }
    }

    #[test]
    fn test_dedup_by_title() {
        let db = test_db();
        add_sample_paper(&db, "Same Title", None);
        let result = db
            .add_paper(AddPaperParams {
                title: "Same Title".to_string(),
                force: false,
                ..Default::default()
            })
            .unwrap();
        match result {
            DedupResult::Duplicate { matched_by, .. } => {
                assert_eq!(matched_by, "title");
            }
            DedupResult::New(_) => panic!("Should be duplicate"),
        }
    }

    #[test]
    fn test_dedup_force() {
        let db = test_db();
        add_sample_paper(&db, "Force Test", Some("2301.00001"));
        let result = db
            .add_paper(AddPaperParams {
                title: "Force Test".to_string(),
                arxiv_id: Some("2301.00001".to_string()),
                force: true,
                ..Default::default()
            })
            .unwrap();
        match result {
            DedupResult::New(p) => assert_eq!(p.title, "Force Test"),
            DedupResult::Duplicate { .. } => panic!("Should be new with --force"),
        }
    }

    #[test]
    fn test_normalize_arxiv_id() {
        assert_eq!(normalize_arxiv_id_pub("2301.07041"), "2301.07041");
        assert_eq!(normalize_arxiv_id_pub("arXiv:2301.07041"), "2301.07041");
        assert_eq!(normalize_arxiv_id_pub("https://arxiv.org/abs/2301.07041"), "2301.07041");
        assert_eq!(normalize_arxiv_id_pub("2301.07041v2"), "2301.07041");
    }

    // ── Phase 2: Tags ──

    #[test]
    fn test_tag_crud() {
        let db = test_db();
        let paper = add_sample_paper(&db, "Tagged Paper", None);

        // add tag
        db.add_paper_tag(&paper.id, "NLP").unwrap();
        let tags = db.get_paper_tags(&paper.id).unwrap();
        assert_eq!(tags, vec!["NLP"]);

        // add another tag
        db.add_paper_tag(&paper.id, "Transformer").unwrap();
        let tags = db.get_paper_tags(&paper.id).unwrap();
        assert!(tags.contains(&"NLP".to_string()));
        assert!(tags.contains(&"Transformer".to_string()));

        // list all tags
        let all = db.list_tags().unwrap();
        assert_eq!(all.len(), 2);

        // remove tag from paper
        db.remove_paper_tag(&paper.id, "NLP").unwrap();
        let tags = db.get_paper_tags(&paper.id).unwrap();
        assert_eq!(tags, vec!["Transformer"]);

        // delete tag entirely (removes "Transformer", "NLP" still exists in tags table)
        assert!(db.delete_tag("Transformer").unwrap());
        let all = db.list_tags().unwrap();
        assert_eq!(all.len(), 1); // NLP still exists
        assert_eq!(all[0].name, "NLP");

        // delete the remaining tag
        assert!(db.delete_tag("NLP").unwrap());
        let all = db.list_tags().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn test_tag_auto_create() {
        let db = test_db();
        let paper = add_sample_paper(&db, "Auto Tag", None);
        // add_paper_tag should auto-create the tag
        db.add_paper_tag(&paper.id, "NewTag").unwrap();
        let all = db.list_tags().unwrap();
        assert!(all.iter().any(|t| t.name == "NewTag"));
    }

    #[test]
    fn test_list_by_tag() {
        let db = test_db();
        let p1 = add_sample_paper(&db, "Paper 1", None);
        let p2 = add_sample_paper(&db, "Paper 2", None);
        db.add_paper_tag(&p1.id, "ML").unwrap();
        db.add_paper_tag(&p2.id, "NLP").unwrap();

        let result = db
            .list_papers(&ListParams {
                tag: Some("ML".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.papers[0].title, "Paper 1");
    }

    // ── Phase 2: Notes ──

    #[test]
    fn test_note_crud() {
        let db = test_db();
        let paper = add_sample_paper(&db, "Note Paper", None);

        // add note
        let note = db
            .add_note(&paper.id, "Key insight about transformers", "summary")
            .unwrap();
        assert_eq!(note.content, "Key insight about transformers");
        assert!(matches!(note.note_type, NoteType::Summary));

        // list notes
        let notes = db.list_notes(&paper.id).unwrap();
        assert_eq!(notes.len(), 1);

        // update note
        let updated = db
            .update_note(&note.id, Some("Updated insight"), None)
            .unwrap();
        assert_eq!(updated.content, "Updated insight");

        // search notes
        let (found, total) = db.search_notes("Updated", 1, 10).unwrap();
        assert_eq!(total, 1);
        assert_eq!(found[0].content, "Updated insight");

        // delete note
        assert!(db.delete_note(&note.id).unwrap());
        assert!(db.list_notes(&paper.id).unwrap().is_empty());
    }

    #[test]
    fn test_note_search_pagination() {
        let db = test_db();
        let paper = add_sample_paper(&db, "Multi Note", None);
        for i in 0..12 {
            db.add_note(&paper.id, &format!("Note about keyword {}", i), "general")
                .unwrap();
        }
        let (page1, total) = db.search_notes("keyword", 1, 5).unwrap();
        assert_eq!(total, 12);
        assert_eq!(page1.len(), 5);
        let (page2, _) = db.search_notes("keyword", 2, 5).unwrap();
        assert_eq!(page2.len(), 5);
    }

    // ── Phase 2: Groups ──

    #[test]
    fn test_group_crud() {
        let db = test_db();
        let paper = add_sample_paper(&db, "Grouped Paper", None);

        // create group
        db.create_group("survey", Some("Papers for survey")).unwrap();
        let groups = db.list_groups().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0.name, "survey");

        // assign paper
        db.add_paper_to_group(&paper.id, "survey").unwrap();
        let groups = db.list_groups().unwrap();
        assert_eq!(groups[0].1, 1);

        // list by group
        let result = db
            .list_papers(&ListParams {
                group: Some("survey".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(result.total, 1);

        // unassign
        db.remove_paper_from_group(&paper.id, "survey").unwrap();
        let groups = db.list_groups().unwrap();
        assert_eq!(groups[0].1, 0);

        // delete group
        assert!(db.delete_group("survey").unwrap());
        assert!(db.list_groups().unwrap().is_empty());
    }

    #[test]
    fn test_group_auto_create_on_assign() {
        let db = test_db();
        let paper = add_sample_paper(&db, "Auto Group", None);
        db.add_paper_to_group(&paper.id, "NewGroup").unwrap();
        let groups = db.list_groups().unwrap();
        assert!(groups.iter().any(|(g, _)| g.name == "NewGroup"));
    }

    // ── Phase 2: Search (FTS5) ──

    #[test]
    fn test_fts5_search() {
        let db = test_db();
        add_sample_paper(&db, "Attention Is All You Need", None);
        add_sample_paper(&db, "BERT Pre-training", None);
        add_sample_paper(&db, "GPT-3 Language Models", None);

        let result = db.search_papers("BERT", None, None, None, 1, 10).unwrap();
        assert_eq!(result.total, 1);
        assert!(result.papers[0].title.contains("BERT"));

        let result = db.search_papers("attention", None, None, None, 1, 10).unwrap();
        assert_eq!(result.total, 1);
    }

    #[test]
    fn test_fts5_search_pagination() {
        let db = test_db();
        for i in 0..12 {
            db.add_paper(AddPaperParams {
                title: format!("Transformer variant {}", i),
                abstract_text: Some("A transformer model".to_string()),
                force: false,
                ..Default::default()
            })
            .unwrap();
        }
        let page1 = db.search_papers("transformer", None, None, None, 1, 5).unwrap();
        assert_eq!(page1.total, 12);
        assert_eq!(page1.papers.len(), 5);
        assert_eq!(page1.total_pages, 3);

        let page3 = db.search_papers("transformer", None, None, None, 3, 5).unwrap();
        assert_eq!(page3.papers.len(), 2);
    }

    #[test]
    fn test_fts5_search_empty() {
        let db = test_db();
        let result = db.search_papers("nonexistent", None, None, None, 1, 10).unwrap();
        assert_eq!(result.total, 0);
        assert!(result.papers.is_empty());
    }

    // ── Phase 3: Citations ──

    #[test]
    fn test_citation_crud() {
        let db = test_db();
        let a = add_sample_paper(&db, "Paper A", None);
        let b = add_sample_paper(&db, "Paper B", None);
        let c = add_sample_paper(&db, "Paper C", None);

        // add citations
        let cite1 = db.add_citation(&a.id, &b.id, "cites", 4, None).unwrap();
        assert_eq!(cite1.from_id, a.id);
        assert_eq!(cite1.to_id, b.id);
        assert!(matches!(cite1.relation, RelationType::Cites));

        let cite2 = db.add_citation(&a.id, &c.id, "related", 3, Some("similar approach")).unwrap();
        assert!(matches!(cite2.relation, RelationType::RelatedTo));

        // list citations
        let cites = db.list_citations(&a.id).unwrap();
        assert_eq!(cites.len(), 2);

        // remove one
        assert!(db.remove_citation(&a.id, &b.id, "cites").unwrap());
        let cites = db.list_citations(&a.id).unwrap();
        assert_eq!(cites.len(), 1);

        // remove nonexistent
        assert!(!db.remove_citation(&a.id, &b.id, "cites").unwrap());
    }

    #[test]
    fn test_citation_graph() {
        let db = test_db();
        let a = add_sample_paper(&db, "Paper A", None);
        let b = add_sample_paper(&db, "Paper B", None);
        let c = add_sample_paper(&db, "Paper C", None);

        db.add_citation(&a.id, &b.id, "cites", 4, None).unwrap();
        db.add_citation(&b.id, &c.id, "cites", 3, None).unwrap();

        // depth 1: A -> B
        let graph = db.citation_graph(&a.id, 1).unwrap();
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.papers.len(), 2); // A and B

        // depth 2: A -> B -> C
        let graph = db.citation_graph(&a.id, 2).unwrap();
        assert!(graph.edges.len() >= 2);
        assert!(graph.papers.len() >= 3);
    }

    #[test]
    fn test_citation_graph_empty() {
        let db = test_db();
        let a = add_sample_paper(&db, "Lonely Paper", None);
        let graph = db.citation_graph(&a.id, 2).unwrap();
        assert!(graph.edges.is_empty());
    }

    // ── Search filtering by tag and group ──

    #[test]
    fn test_search_by_tag() {
        let db = test_db();
        let p1 = add_sample_paper(&db, "Deep Learning Basics", None);
        let p2 = add_sample_paper(&db, "Deep Learning Advanced", None);
        let p3 = add_sample_paper(&db, "Natural Language Processing", None);

        db.add_paper_tag(&p1.id, "ML").unwrap();
        db.add_paper_tag(&p2.id, "ML").unwrap();
        db.add_paper_tag(&p3.id, "NLP").unwrap();

        // Search for "deep" filtered by tag "ML" -> 2 results
        let result = db.search_papers("deep", None, Some("ML"), None, 1, 10).unwrap();
        assert_eq!(result.total, 2);

        // Search for "deep" filtered by tag "NLP" -> 0 results
        let result = db.search_papers("deep", None, Some("NLP"), None, 1, 10).unwrap();
        assert_eq!(result.total, 0);

        // Search for "deep" filtered by nonexistent tag -> 0 results
        let result = db.search_papers("deep", None, Some("nonexistent"), None, 1, 10).unwrap();
        assert_eq!(result.total, 0);
    }

    #[test]
    fn test_search_by_group() {
        let db = test_db();
        let p1 = add_sample_paper(&db, "Transformer Architecture", None);
        let p2 = add_sample_paper(&db, "Transformer Applications", None);
        let p3 = add_sample_paper(&db, "CNN Architecture", None);

        db.add_paper_to_group(&p1.id, "survey").unwrap();
        db.add_paper_to_group(&p2.id, "survey").unwrap();
        db.add_paper_to_group(&p3.id, "project").unwrap();

        // Search for "transformer" filtered by group "survey" -> 2 results
        let result = db.search_papers("transformer", None, None, Some("survey"), 1, 10).unwrap();
        assert_eq!(result.total, 2);

        // Search for "architecture" filtered by group "survey" -> 1 result (Transformer Architecture)
        let result = db.search_papers("architecture", None, None, Some("survey"), 1, 10).unwrap();
        assert_eq!(result.total, 1);
        assert!(result.papers[0].title.contains("Transformer"));

        // Search for "architecture" filtered by group "project" -> 1 result (CNN Architecture)
        let result = db.search_papers("architecture", None, None, Some("project"), 1, 10).unwrap();
        assert_eq!(result.total, 1);
        assert!(result.papers[0].title.contains("CNN"));

        // Search for "transformer" filtered by nonexistent group -> 0 results
        let result = db.search_papers("transformer", None, None, Some("nonexistent"), 1, 10).unwrap();
        assert_eq!(result.total, 0);
    }

    // ── Phase: Multi-keyword search parser ──

    #[test]
    fn test_search_single_word_prefix() {
        let db = test_db();
        add_sample_paper(&db, "Attention Is All You Need", None);
        add_sample_paper(&db, "BERT Pre-training", None);
        add_sample_paper(&db, "GPT-3 Language Models", None);

        let result = db.search_papers("BERT", None, None, None, 1, 10).unwrap();
        assert_eq!(result.total, 1);
        assert!(result.papers[0].title.contains("BERT"));
    }

    #[test]
    fn test_search_multi_keyword_and() {
        let db = test_db();
        db.add_paper(AddPaperParams {
            title: "Deep Learning for NLP".to_string(),
            abstract_text: Some("Deep neural networks for natural language processing".to_string()),
            force: false,
            ..Default::default()
        }).unwrap();
        db.add_paper(AddPaperParams {
            title: "Deep Reinforcement Learning".to_string(),
            abstract_text: Some("Learning from rewards in deep networks".to_string()),
            force: false,
            ..Default::default()
        }).unwrap();
        db.add_paper(AddPaperParams {
            title: "Shallow Learning Baseline".to_string(),
            abstract_text: Some("A simple baseline method".to_string()),
            force: false,
            ..Default::default()
        }).unwrap();

        // "deep learning" should match both papers with "deep" AND "learning"
        let result = db.search_papers("deep learning", None, None, None, 1, 10).unwrap();
        assert_eq!(result.total, 2);
    }

    #[test]
    fn test_search_phrase() {
        let db = test_db();
        db.add_paper(AddPaperParams {
            title: "The Attention Mechanism in Transformers".to_string(),
            abstract_text: Some("We study the attention mechanism".to_string()),
            force: false,
            ..Default::default()
        }).unwrap();
        db.add_paper(AddPaperParams {
            title: "Mechanism of Self-Attention".to_string(),
            abstract_text: Some("Attention patterns in neural nets".to_string()),
            force: false,
            ..Default::default()
        }).unwrap();

        // "attention mechanism" as a phrase should only match the first
        let result = db.search_papers("\"attention mechanism\"", None, None, None, 1, 10).unwrap();
        assert_eq!(result.total, 1);
        assert!(result.papers[0].title.contains("Attention Mechanism in"));
    }

    #[test]
    fn test_search_exclude() {
        let db = test_db();
        db.add_paper(AddPaperParams {
            title: "Deep Learning for Vision".to_string(),
            abstract_text: Some("Convolutional networks".to_string()),
            force: false,
            ..Default::default()
        }).unwrap();
        db.add_paper(AddPaperParams {
            title: "Deep Learning for NLP".to_string(),
            abstract_text: Some("Transformer models".to_string()),
            force: false,
            ..Default::default()
        }).unwrap();
        db.add_paper(AddPaperParams {
            title: "Shallow Methods".to_string(),
            abstract_text: None,
            force: false,
            ..Default::default()
        }).unwrap();

        // "deep -nlp" should match "Deep Learning for Vision" but not "Deep Learning for NLP"
        let result = db.search_papers("deep -nlp", None, None, None, 1, 10).unwrap();
        assert_eq!(result.total, 1);
        assert!(result.papers[0].title.contains("Vision"));
    }

    #[test]
    fn test_search_or() {
        let db = test_db();
        db.add_paper(AddPaperParams {
            title: "Transformer Architecture".to_string(),
            abstract_text: None,
            force: false,
            ..Default::default()
        }).unwrap();
        db.add_paper(AddPaperParams {
            title: "Attention Mechanism".to_string(),
            abstract_text: None,
            force: false,
            ..Default::default()
        }).unwrap();
        db.add_paper(AddPaperParams {
            title: "Convolutional Networks".to_string(),
            abstract_text: None,
            force: false,
            ..Default::default()
        }).unwrap();

        let result = db.search_papers("transformer OR attention", None, None, None, 1, 10).unwrap();
        assert_eq!(result.total, 2);
    }

    #[test]
    fn test_search_prefix_wildcard() {
        let db = test_db();
        db.add_paper(AddPaperParams {
            title: "Transformers for NLP".to_string(),
            abstract_text: None,
            force: false,
            ..Default::default()
        }).unwrap();
        db.add_paper(AddPaperParams {
            title: "Transformation Rules".to_string(),
            abstract_text: None,
            force: false,
            ..Default::default()
        }).unwrap();
        db.add_paper(AddPaperParams {
            title: "Attention Model".to_string(),
            abstract_text: None,
            force: false,
            ..Default::default()
        }).unwrap();

        let result = db.search_papers("transform*", None, None, None, 1, 10).unwrap();
        assert_eq!(result.total, 2);
    }
}