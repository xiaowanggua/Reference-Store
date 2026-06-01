use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use refstore_core::{Database, model::AddPaperParams};

pub fn run(db: &Database, path: &str) -> Result<()> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))?;

    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "bib" => import_bibtex(db, &content),
        "json" => import_json_backup(db, &content),
        _ => anyhow::bail!("Unsupported file format: .{}. Supported: .bib, .json", ext),
    }
}

fn import_bibtex(db: &Database, content: &str) -> Result<()> {
    let entries = parse_bibtex(content);
    if entries.is_empty() {
        println!("No entries found in BibTeX file.");
        return Ok(());
    }

    let mut added = 0u32;
    let mut skipped = 0u32;

    for entry in &entries {
        let params = AddPaperParams {
            title: entry.title.clone(),
            authors: Some(entry.authors.clone()),
            abstract_text: entry.abstract_text.clone(),
            source_url: entry.url.clone(),
            doi: entry.doi.clone(),
            arxiv_id: None, // BibTeX doesn't usually have arxiv_id directly
            pdf_path: None,
            publish_date: None,
            venue: entry.venue.clone(),
            force: false,
        };

        match db.add_paper(params)? {
            refstore_core::model::DedupResult::New(p) => {
                added += 1;
                println!("  Added: {} ({})", p.title, &p.id[..8]);
            }
            refstore_core::model::DedupResult::Duplicate { existing, matched_by } => {
                skipped += 1;
                println!(
                    "  Skipped (dup by {}): {} ({})",
                    matched_by, existing.title, &existing.id[..8]
                );
            }
        }
    }

    println!("\nImport complete: {} added, {} duplicates skipped.", added, skipped);
    Ok(())
}

/// Minimal BibTeX entry
struct BibEntry {
    title: String,
    authors: Vec<String>,
    abstract_text: Option<String>,
    doi: Option<String>,
    url: Option<String>,
    venue: Option<String>,
}

/// 最简 BibTeX 解析器（不依赖外部 crate，处理常见格式）
fn parse_bibtex(content: &str) -> Vec<BibEntry> {
    let mut entries = Vec::new();

    for entry_block in split_entries(content) {
        let title = extract_field(&entry_block, "title");
        let author_str = extract_field(&entry_block, "author");
        let authors = parse_bibtex_authors(&author_str);
        let abstract_text = extract_field_optional(&entry_block, "abstract");
        let doi = extract_field_optional(&entry_block, "doi");
        let url = extract_field_optional(&entry_block, "url");
        let venue = extract_field_optional(&entry_block, "journal")
            .or_else(|| extract_field_optional(&entry_block, "booktitle"));

        if !title.is_empty() {
            entries.push(BibEntry {
                title,
                authors,
                abstract_text,
                doi,
                url,
                venue,
            });
        }
    }

    entries
}

fn split_entries(content: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut current = String::new();
    let mut brace_depth = 0;
    let mut in_entry = false;

    for ch in content.chars() {
        if ch == '@' && brace_depth == 0 {
            if !current.is_empty() {
                entries.push(current.clone());
            }
            current.clear();
            in_entry = true;
        }
        if in_entry {
            current.push(ch);
            if ch == '{' {
                brace_depth += 1;
            } else if ch == '}' {
                brace_depth -= 1;
                if brace_depth == 0 {
                    entries.push(current.clone());
                    current.clear();
                    in_entry = false;
                }
            }
        }
    }

    entries
}

fn extract_field(entry: &str, field: &str) -> String {
    extract_field_optional(entry, field).unwrap_or_default()
}

fn extract_field_optional(entry: &str, field: &str) -> Option<String> {
    // Case-insensitive search for "field = "
    let entry_lower = entry.to_lowercase();
    let field_lower = format!("{} ", field.to_lowercase());
    let search = format!("{}=", field_lower);

    let pos = entry_lower.find(&search)?;
    let rest = &entry[pos + search.len()..];
    let rest = rest.trim_start();

    let value = if rest.starts_with('{') {
        extract_braced(rest)
    } else if rest.starts_with('"') {
        extract_quoted(rest)
    } else {
        rest.split(',').next().unwrap_or("").trim().to_string()
    };
    let cleaned = clean_latex(&value);
    if cleaned.is_empty() { None } else { Some(cleaned) }
}

fn extract_braced(s: &str) -> String {
    let mut depth = 0;
    let mut result = String::new();
    for ch in s.chars() {
        match ch {
            '{' => {
                if depth > 0 { result.push(ch); }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 { break; }
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

fn extract_quoted(s: &str) -> String {
    let s = &s[1..]; // skip opening quote
    s.split('"').next().unwrap_or("").to_string()
}

fn clean_latex(s: &str) -> String {
    s.replace("\\", "")
     .replace("{", "")
     .replace("}", "")
     .trim()
     .to_string()
}

fn parse_bibtex_authors(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![];
    }
    s.split(" and ")
     .map(|a| a.split(',').next().unwrap_or(a).trim().to_string())
     .filter(|a| !a.is_empty())
     .collect()
}

fn import_json_backup(db: &Database, content: &str) -> Result<()> {
    let backup: refstore_core::BackupData = serde_json::from_str(content)
        .context("Invalid JSON backup file")?;

    let stats = db.import_backup(&backup)?;

    println!("JSON backup import complete:");
    println!("  Papers:     {} imported, {} skipped", stats.papers_imported, stats.papers_skipped);
    println!("  Tags:       {} imported, {} skipped", stats.tags_imported, stats.tags_skipped);
    println!("  Groups:     {} imported, {} skipped", stats.groups_imported, stats.groups_skipped);
    println!("  Notes:      {} imported, {} skipped", stats.notes_imported, stats.notes_skipped);
    println!("  Citations:  {} imported, {} skipped", stats.citations_imported, stats.citations_skipped);
    Ok(())
}
