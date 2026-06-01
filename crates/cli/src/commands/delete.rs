use anyhow::Result;
use refstore_core::Database;

pub fn run(db: &Database, id: &str) -> Result<()> {
    let deleted = db.delete_paper(id)?;
    if deleted {
        println!("Deleted: {}", id);
    } else {
        println!("Paper not found: {}", id);
    }
    Ok(())
}
