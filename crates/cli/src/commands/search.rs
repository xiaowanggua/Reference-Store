use anyhow::Result;
use refstore_core::Database;

use crate::output::format_paper_list;
use crate::SearchArgs;

pub fn run(db: &Database, args: &SearchArgs) -> Result<()> {
    let result = db.search_papers(
        &args.query,
        Some(args.r#in.as_str()),
        args.tag.as_deref(),
        args.group.as_deref(),
        args.page,
        args.page_size,
    )?;

    let output = format_paper_list(&result);
    println!("{}", output);
    Ok(())
}
