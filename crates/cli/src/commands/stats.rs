use anyhow::Result;
use refstore_core::Database;

pub fn run(db: &Database) -> Result<()> {
    let total = db.count_papers(None, None, None)?;
    let read = db.count_papers(None, None, Some(true))?;
    let unread = db.count_papers(None, None, Some(false))?;

    println!("Refstore Statistics");
    println!("===================");
    println!("Total papers:  {}", total);
    println!("Read:          {}", read);
    println!("Unread:        {}", unread);

    // Monthly trend
    let trend = db.monthly_trend()?;
    if !trend.is_empty() {
        println!("\nMonthly Trend:");
        let max_count = trend.iter().map(|(_, c)| *c).max().unwrap_or(1);
        for (month, count) in &trend {
            let bar_len = if max_count > 0 { (*count as f64 / max_count as f64 * 20.0).ceil() as usize } else { 0 };
            let bar = "█".repeat(bar_len);
            println!("  {}  {:>3} {}", month, count, bar);
        }
    }

    let tags = db.list_tags()?;
    if !tags.is_empty() {
        println!("\nTag Cloud ({} tags):", tags.len());
        for tag in &tags {
            let count = tag.paper_count.unwrap_or(0);
            // Visual weight based on paper count
            let bar = "#".repeat(count.clamp(0, 20) as usize);
            let alias_str = if tag.aliases.is_empty() {
                String::new()
            } else {
                format!(" [{}]", tag.aliases.join(", "))
            };
            println!("  {}{} ({} papers) {}", tag.name, alias_str, count, bar);
        }
    }

    let groups = db.list_groups()?;
    if !groups.is_empty() {
        println!("\nGroups ({}):", groups.len());
        for (group, count) in &groups {
            println!("  {} ({} papers)", group.name, count);
        }
    }

    Ok(())
}
