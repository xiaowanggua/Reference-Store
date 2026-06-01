use anyhow::Result;
use refstore_core::Database;

use crate::CiteAction;

pub fn run(db: &Database, action: &CiteAction) -> Result<()> {
    match action {
        CiteAction::Add { from, to, relation, strength, note } => {
            let cite = db.add_citation(from, to, relation, *strength, note.as_deref())?;
            println!(
                "Added: {} --[{}]--> {} (strength: {})",
                &cite.from_id[..8],
                cite.relation.as_str(),
                &cite.to_id[..8],
                cite.strength,
            );
        }
        CiteAction::Remove { from, to, relation } => {
            let removed = db.remove_citation(from, to, relation)?;
            if removed {
                println!("Citation removed.");
            } else {
                println!("Citation not found.");
            }
        }
        CiteAction::List { id } => {
            let citations = db.list_citations(id)?;
            if citations.is_empty() {
                println!("No citations for this paper.");
                return Ok(());
            }
            println!("Citations:");
            for c in &citations {
                let direction = if c.from_id.starts_with(&id[..8.min(id.len())]) {
                    format!("--> {}", &c.to_id[..8])
                } else {
                    format!("<-- {}", &c.from_id[..8])
                };
                println!(
                    "  {} [{}] (strength: {})",
                    direction,
                    c.relation.as_str(),
                    c.strength,
                );
            }
        }
        CiteAction::Graph { id, depth, format } => {
            let graph = db.citation_graph(id, *depth)?;
            if graph.edges.is_empty() {
                println!("No citation graph for this paper.");
                return Ok(());
            }

            match format.as_str() {
                "mermaid" => print_mermaid(&graph),
                _ => print_text_graph(&graph),
            }
        }
    }
    Ok(())
}

fn print_text_graph(graph: &refstore_core::GraphResult) {
    println!("Citation Graph ({} papers, {} edges):\n", graph.papers.len(), graph.edges.len());

    // Build a quick lookup for short titles
    let title_of = |id: &str| -> String {
        graph
            .papers
            .iter()
            .find(|p| p.id == id)
            .map(|p| truncate(&p.title, 40))
            .unwrap_or_else(|| id[..8].to_string())
    };

    for edge in &graph.edges {
        println!(
            "  {} --[{}]--> {}",
            title_of(&edge.from_id),
            edge.relation.as_str(),
            title_of(&edge.to_id),
        );
    }
}

fn print_mermaid(graph: &refstore_core::GraphResult) {
    let backticks = "```";
    println!("{}mermaid", backticks);
    println!("graph LR");

    for paper in &graph.papers {
        println!("    {}[\"{}\"]", &paper.id[..8], truncate(&paper.title, 30));
    }
    for edge in &graph.edges {
        let label = match &edge.note {
            Some(n) => format!("{}|{}|", edge.relation.as_str(), truncate(n, 20)),
            None => edge.relation.as_str().to_string(),
        };
        println!(
            "    {} -->|{}| {}",
            &edge.from_id[..8],
            label,
            &edge.to_id[..8],
        );
    }
    println!("{}", backticks);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}
