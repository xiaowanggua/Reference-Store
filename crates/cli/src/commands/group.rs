use anyhow::Result;
use refstore_core::Database;

use crate::GroupAction;

pub fn run(db: &Database, action: &GroupAction) -> Result<()> {
    match action {
        GroupAction::Add { name, description } => {
            db.create_group(name, description.as_deref())?;
            println!("Created group: {}", name);
        }
        GroupAction::Delete { name } => {
            let deleted = db.delete_group(name)?;
            if deleted {
                println!("Deleted group: {}", name);
            } else {
                println!("Group not found: {}", name);
            }
        }
        GroupAction::List => {
            let groups = db.list_groups()?;
            if groups.is_empty() {
                println!("No groups yet.");
                return Ok(());
            }
            println!("Groups:");
            for (group, count) in &groups {
                println!("  {} ({} papers)", group.name, count);
            }
        }
        GroupAction::Assign { id, group } => {
            db.add_paper_to_group(id, group)?;
            println!("Added {} to group '{}'", &id[..8.min(id.len())], group);
        }
        GroupAction::Unassign { id, group } => {
            db.remove_paper_from_group(id, group)?;
            println!("Removed {} from group '{}'", &id[..8.min(id.len())], group);
        }
    }
    Ok(())
}
