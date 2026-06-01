use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Table, ContentArrangement};

use refstore_core::model::{DedupResult, ListResult, Paper};

/// 格式化单个论文详情
pub fn format_paper_detail(paper: &Paper) -> String {
    let mut lines = Vec::new();
    lines.push(format!("ID:       {}", paper.id));
    lines.push(format!("Title:    {}", paper.title));

    if !paper.authors.is_empty() {
        lines.push(format!("Authors:  {}", paper.authors.join(", ")));
    }
    if let Some(ref abs) = paper.abstract_text {
        lines.push(format!("Abstract: {}", abs));
    }
    if let Some(ref url) = paper.source_url {
        lines.push(format!("URL:      {}", url));
    }
    if let Some(ref doi) = paper.doi {
        lines.push(format!("DOI:      {}", doi));
    }
    if let Some(ref arxiv) = paper.arxiv_id {
        lines.push(format!("arXiv:    {}", arxiv));
    }
    if let Some(ref path) = paper.pdf_path {
        lines.push(format!("PDF:      {}", path));
    }
    if let Some(ref date) = paper.publish_date {
        lines.push(format!("Date:     {}", date));
    }
    if let Some(ref venue) = paper.venue {
        lines.push(format!("Venue:    {}", venue));
    }

    lines.push(format!("Read:     {}", if paper.is_read { "Yes" } else { "No" }));
    lines.push(format!("Created:  {}", paper.created_at));
    lines.push(format!("Updated:  {}", paper.updated_at));

    lines.join("\n")
}

/// 格式化论文列表（表格）
pub fn format_paper_list(result: &ListResult) -> String {
    if result.papers.is_empty() {
        return "No papers found.".to_string();
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.set_header(vec!["#", "ID (short)", "Title", "Authors", "Read", "Date"]);

    for (i, paper) in result.papers.iter().enumerate() {
        let short_id = &paper.id[..8];
        let authors = if paper.authors.is_empty() {
            "-".to_string()
        } else if paper.authors.len() <= 2 {
            paper.authors.join(", ")
        } else {
            format!("{}, ...", paper.authors[0])
        };
        let read = if paper.is_read { "Y" } else { "N" };
        let date = paper.publish_date.as_deref().unwrap_or("-");

        table.add_row(vec![
            ((result.page - 1) * result.page_size + i as u32 + 1).to_string(),
            short_id.to_string(),
            truncate(&paper.title, 50),
            truncate(&authors, 25),
            read.to_string(),
            date.to_string(),
        ]);
    }

    let mut output = table.to_string();
    output.push_str(&format!(
        "\nPage {} of {} ({} total papers, {} per page)\n",
        result.page, result.total_pages, result.total, result.page_size
    ));
    output
}

/// 精简格式（仅 ID + 标题）
pub fn format_paper_compact(result: &ListResult) -> String {
    if result.papers.is_empty() {
        return "No papers found.".to_string();
    }

    let mut lines = Vec::new();
    for (i, paper) in result.papers.iter().enumerate() {
        lines.push(format!("{}. {}  {}", i + 1, &paper.id[..8], paper.title));
    }
    lines.push(String::new());
    lines.push(format!(
        "Page {} of {} ({} total)",
        result.page, result.total_pages, result.total
    ));
    lines.join("\n")
}

/// 格式化添加结果（含查重提示）
pub fn format_add_result(result: &DedupResult) -> String {
    match result {
        DedupResult::New(paper) => {
            format!("Added: {} ({})", paper.title, &paper.id[..8])
        }
        DedupResult::Duplicate {
            existing,
            matched_by,
        } => {
            format!(
                "Duplicate detected (matched by {}): {} ({})\nUse --force to add anyway.",
                matched_by, existing.title, &existing.id[..8]
            )
        }
    }
}

/// 截断字符串
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
