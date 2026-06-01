use anyhow::Result;
use refstore_core::Database;

use crate::ExportArgs;

pub fn run(db: &Database, args: &ExportArgs) -> Result<()> {
    match args.format.as_str() {
        "bibtex" => export_bibtex(db, args),
        "markdown" => export_markdown(db, args),
        "mermaid" => export_mermaid(db, args),
        "json" => export_json(db, args),
        _ => {
            let msg = format!(
                "Unknown format: {}. Supported: bibtex, markdown, mermaid, json",
                args.format
            );
            anyhow::bail!("{}", msg)
        }
    }
}

fn export_bibtex(db: &Database, args: &ExportArgs) -> Result<()> {
    let papers = get_papers(db, args)?;

    for paper in &papers {
        let key = paper.id[..8].replace('-', "_");
        println!("@article{{{}}},", key);
        println!("  title = {{{}}},", paper.title);
        if !paper.authors.is_empty() {
            println!("  author = {{{}}},", paper.authors.join(" and "));
        }
        if let Some(ref abs) = paper.abstract_text {
            println!("  abstract = {{{}}},", abs);
        }
        if let Some(ref doi) = paper.doi {
            println!("  doi = {{{}}},", doi);
        }
        if let Some(ref url) = paper.source_url {
            println!("  url = {{{}}},", url);
        }
        if let Some(ref date) = paper.publish_date {
            println!("  year = {{{}}},", &date[..4]);
        }
        if let Some(ref venue) = paper.venue {
            println!("  journal = {{{}}},", venue);
        }
        println!("}}");
        println!();
    }

    println!("Exported {} papers as BibTeX.", papers.len());
    Ok(())
}

fn export_markdown(db: &Database, args: &ExportArgs) -> Result<()> {
    let papers = get_papers(db, args)?;

    println!("# Paper Library");
    println!();

    for (i, paper) in papers.iter().enumerate() {
        println!("## {}. {}", i + 1, paper.title);
        if !paper.authors.is_empty() {
            println!("**Authors:** {}", paper.authors.join(", "));
            println!();
        }
        if let Some(ref abs) = paper.abstract_text {
            println!("> {}", abs);
            println!();
        }
        if let Some(ref url) = paper.source_url {
            println!("[Link]({})", url);
            println!();
        }
        if let Some(ref doi) = paper.doi {
            println!("DOI: {}", doi);
            println!();
        }
        let tags = db.get_paper_tags(&paper.id)?;
        if !tags.is_empty() {
            println!("Tags: {}", tags.join(", "));
            println!();
        }
        let status = if paper.is_read { "Yes" } else { "No" };
        println!("Read: {}  |  Added: {}", status, paper.created_at);
        println!();
        println!("---");
        println!();
    }

    println!("Exported {} papers as Markdown.", papers.len());
    Ok(())
}

fn export_mermaid(db: &Database, args: &ExportArgs) -> Result<()> {
    let paper_id = args.graph.as_deref().or(args.id.as_deref());
    let id = paper_id.ok_or_else(|| {
        anyhow::anyhow!("--graph <paper-id> or --id <paper-id> required for mermaid export")
    })?;

    let graph = db.citation_graph(id, 3)?;
    if graph.edges.is_empty() {
        println!("No citation graph found.");
        return Ok(());
    }

    // Use raw string to avoid backtick issues
    let backticks = "```";
    println!("{}mermaid", backticks);
    println!("graph LR");
    for paper in &graph.papers {
        let short_id = &paper.id[..8];
        let title = truncate(&paper.title, 30);
        println!("    {}[\"{}\"]", short_id, title);
    }
    for edge in &graph.edges {
        let from = &edge.from_id[..8];
        let to = &edge.to_id[..8];
        println!("    {} -->|{}| {}", from, edge.relation.as_str(), to);
    }
    println!("{}", backticks);
    Ok(())
}

fn export_json(db: &Database, args: &ExportArgs) -> Result<()> {
    let papers = get_papers(db, args)?;
    let json = serde_json::to_string_pretty(&papers)?;
    println!("{}", json);
    println!();
    println!("Exported {} papers as JSON.", papers.len());
    Ok(())
}

fn get_papers(db: &Database, args: &ExportArgs) -> Result<Vec<refstore_core::model::Paper>> {
    if let Some(ref id) = args.id {
        let paper =
            db.get_paper(id)?.ok_or_else(|| anyhow::anyhow!("Paper not found: {}", id))?;
        return Ok(vec![paper]);
    }

    let mut params = refstore_core::model::ListParams {
        tag: args.tag.clone(),
        ..Default::default()
    };
    params.page_size = 10000;
    let result = db.list_papers(&params)?;
    Ok(result.papers)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
