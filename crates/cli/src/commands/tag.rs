use std::collections::HashMap;

use anyhow::Result;
use refstore_core::Database;

use crate::TagAction;

pub fn run(db: &Database, action: &TagAction) -> Result<()> {
    match action {
        TagAction::Add { id, name, parent, alias } => {
            // Resolve parent tag id if --parent is given
            let parent_id: Option<String> = if let Some(parent_name) = parent {
                let tags = db.list_tags()?;
                tags.iter()
                    .find(|t| &t.name == parent_name)
                    .map(|t| t.id.clone())
            } else {
                None
            };

            // Try to create the tag with parent and alias; if it already exists, just use it
            let aliases_opt: Option<Vec<String>> = alias.as_ref().map(|a| vec![a.clone()]);
            let _ = db.create_tag(
                name,
                parent_id.as_deref(),
                None,
                aliases_opt,
            );

            db.add_paper_tag(id, name)?;
            println!("Tagged {} with '{}'", &id[..8.min(id.len())], name);
        }
        TagAction::Remove { id, name } => {
            db.remove_paper_tag(id, name)?;
            println!("Removed tag '{}' from {}", name, &id[..8.min(id.len())]);
        }
        TagAction::List => {
            let tags = db.list_tags()?;
            if tags.is_empty() {
                println!("No tags yet.");
                return Ok(());
            }
            print_tag_tree(&tags);
        }
        TagAction::Delete { name } => {
            let deleted = db.delete_tag(name)?;
            if deleted {
                println!("Deleted tag: {}", name);
            } else {
                println!("Tag not found: {}", name);
            }
        }
    }
    Ok(())
}

/// Print tags in a tree structure based on parent_id
fn print_tag_tree(tags: &[refstore_core::model::Tag]) {
    // Map tag id -> tag reference
    let id_map: HashMap<&str, &refstore_core::model::Tag> = tags
        .iter()
        .map(|t| (t.id.as_str(), t))
        .collect();

    // Map parent_id -> list of child tags
    let mut children_map: HashMap<Option<&str>, Vec<&refstore_core::model::Tag>> = HashMap::new();
    for tag in tags {
        children_map
            .entry(tag.parent_id.as_deref())
            .or_default()
            .push(tag);
    }

    println!("Tags:");

    // Print root tags (parent_id = None)
    let roots = children_map.get(&None).cloned().unwrap_or_default();
    for root in &roots {
        print_tree_node(&root.id, &id_map, &children_map, 1);
    }

    // Print orphan tags (parent_id points to non-existent tag)
    for tag in tags {
        if let Some(ref pid) = tag.parent_id {
            if !id_map.contains_key(pid.as_str()) && pid != &tag.id {
                let count_str = tag.paper_count.map(|c| format!(" ({} papers)", c)).unwrap_or_default();
                println!("  {}{}", tag.name, count_str);
            }
        }
    }
}

fn print_tree_node(
    tag_id: &str,
    id_map: &HashMap<&str, &refstore_core::model::Tag>,
    children_map: &HashMap<Option<&str>, Vec<&refstore_core::model::Tag>>,
    depth: usize,
) {
    if let Some(tag) = id_map.get(tag_id) {
        let indent = "  ".repeat(depth);
        let count_str = tag.paper_count.map(|c| format!(" ({} papers)", c)).unwrap_or_default();
        let alias_str = if tag.aliases.is_empty() {
            String::new()
        } else {
            format!(" [aliases: {}]", tag.aliases.join(", "))
        };
        println!("{}{}{}{}", indent, tag.name, alias_str, count_str);

        // Recurse into children
        if let Some(children) = children_map.get(&Some(tag.id.as_str())) {
            for child in children {
                print_tree_node(&child.id, id_map, children_map, depth + 1);
            }
        }
    }
}
