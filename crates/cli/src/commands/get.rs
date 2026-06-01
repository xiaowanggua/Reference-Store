use anyhow::Result;
use refstore_core::Database;

use crate::output::format_paper_detail;

pub fn run(db: &Database, id: &str, format: &str) -> Result<()> {
    let paper = db
        .get_paper(id)?
        .ok_or_else(|| anyhow::anyhow!("Paper not found: {}", id))?;

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&paper)?);
        }
        _ => {
            println!("{}", format_paper_detail(&paper));
        }
    }

    Ok(())
}
