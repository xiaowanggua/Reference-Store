use anyhow::{Context, Result};
use chrono::NaiveDate;
use refstore_core::{fetch_by_arxiv, fetch_by_doi, Database, model::AddPaperParams};

use crate::output::format_add_result;

pub fn run(db: &Database, args: &crate::AddArgs) -> Result<()> {
    // If --doi provided, try to fetch metadata first
    let params = if let Some(ref doi) = args.doi {
        match fetch_by_doi(doi) {
            Ok(mut p) => {
                // Override with any explicitly provided args
                apply_overrides(&mut p, args);
                p.force = args.force;
                p
            }
            Err(e) => {
                eprintln!("Warning: DOI fetch failed ({}), using manual entry.", e);
                build_manual_params(args)?
            }
        }
    } else if let Some(ref arxiv) = args.arxiv {
        match fetch_by_arxiv(arxiv) {
            Ok(mut p) => {
                apply_overrides(&mut p, args);
                p.force = args.force;
                p
            }
            Err(e) => {
                eprintln!("Warning: arXiv fetch failed ({}), using manual entry.", e);
                build_manual_params(args)?
            }
        }
    } else {
        build_manual_params(args)?
    };

    let result = db.add_paper(params)?;
    println!("{}", format_add_result(&result));
    Ok(())
}

fn build_manual_params(args: &crate::AddArgs) -> Result<AddPaperParams> {
    let publish_date = args
        .date
        .as_deref()
        .map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d"))
        .transpose()
        .context("Invalid date format, expected YYYY-MM-DD")?;

    let authors = args.authors.as_deref().map(|s| {
        s.split(',')
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty())
            .collect::<Vec<_>>()
    });

    Ok(AddPaperParams {
        title: args
            .title
            .clone()
            .context("--title is required when not using --doi or --arxiv")?,
        authors,
        abstract_text: args.abstract_text.clone(),
        source_url: args.url.clone(),
        doi: args.doi.clone(),
        arxiv_id: args.arxiv.clone(),
        pdf_path: args.pdf.clone(),
        publish_date,
        venue: args.venue.clone(),
        force: args.force,
    })
}

fn apply_overrides(params: &mut AddPaperParams, args: &crate::AddArgs) {
    if args.title.is_some() { params.title = args.title.clone().unwrap(); }
    if args.authors.is_some() {
        params.authors = args.authors.as_deref().map(|s| {
            s.split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect::<Vec<_>>()
        });
    }
    if args.abstract_text.is_some() { params.abstract_text = args.abstract_text.clone(); }
    if args.url.is_some() { params.source_url = args.url.clone(); }
    if args.pdf.is_some() { params.pdf_path = args.pdf.clone(); }
    if args.venue.is_some() { params.venue = args.venue.clone(); }
    if args.date.is_some() {
        params.publish_date = args.date.as_deref()
            .map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d"))
            .transpose()
            .ok()
            .flatten();
    }
}
