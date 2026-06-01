use anyhow::Result;
use refstore_core::{Database, model::{ListParams, SortField}};

use crate::output::{format_paper_compact, format_paper_list};

pub fn run(db: &Database, args: &crate::ListArgs) -> Result<()> {
    let sort = match args.sort.as_str() {
        "title" => SortField::Title,
        "date" => SortField::Date,
        _ => SortField::Created,
    };

    let params = ListParams {
        page: args.page,
        page_size: args.page_size,
        tag: args.tag.clone(),
        group: args.group.clone(),
        is_read: args.status.as_deref().and_then(|s| match s {
            "read" => Some(true),
            "unread" => Some(false),
            _ => None,
        }),
        sort,
    };

    let result = db.list_papers(&params)?;

    match args.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&result.papers)?);
        }
        "compact" => {
            println!("{}", format_paper_compact(&result));
        }
        _ => {
            println!("{}", format_paper_list(&result));
        }
    }

    Ok(())
}
