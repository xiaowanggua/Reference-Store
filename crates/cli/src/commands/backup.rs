use std::fs;

use anyhow::{Context, Result};
use refstore_core::Database;

pub fn run(db: &Database, path: &str) -> Result<()> {
    let backup = db.export_backup()?;
    let json = serde_json::to_string_pretty(&backup)?;

    fs::write(path, &json)
        .with_context(|| format!("Failed to write backup to {}", path))?;

    println!("Backup saved to {}", path);
    println!("  Papers:     {}", backup.papers.len());
    println!("  Tags:       {}", backup.tags.len());
    println!("  Groups:     {}", backup.groups.len());
    println!("  Notes:      {}", backup.notes.len());
    println!("  Citations:  {}", backup.citations.len());
    Ok(())
}
