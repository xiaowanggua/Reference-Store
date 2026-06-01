use anyhow::Result;
use refstore_core::Database;

use crate::NoteAction;

pub fn run(db: &Database, action: &NoteAction) -> Result<()> {
    match action {
        NoteAction::Add { id, content, note_type } => {
            let note = db.add_note(id, content, note_type)?;
            println!("Note added ({}): {}", note.note_type.as_str(), &note.id[..8]);
        }
        NoteAction::List { id } => {
            let notes = db.list_notes(id)?;
            if notes.is_empty() {
                println!("No notes for this paper.");
                return Ok(());
            }
            for note in &notes {
                println!("---");
                println!("ID:       {}", &note.id[..8]);
                println!("Type:     {}", note.note_type.as_str());
                println!("Content:  {}", note.content);
                println!("Updated:  {}", note.updated_at);
            }
        }
        NoteAction::Update { id, content, note_type } => {
            let note = db.update_note(id, content.as_deref(), note_type.as_deref())?;
            println!("Note updated: {}", &note.id[..8]);
        }
        NoteAction::Delete { id } => {
            let deleted = db.delete_note(id)?;
            if deleted {
                println!("Note deleted: {}", id);
            } else {
                println!("Note not found: {}", id);
            }
        }
        NoteAction::Search { keyword, page, page_size } => {
            let (notes, total) = db.search_notes(keyword, *page, *page_size)?;
            if notes.is_empty() {
                println!("No notes matching '{}'.", keyword);
                return Ok(());
            }
            println!("Found {} notes matching '{}':\n", total, keyword);
            for note in &notes {
                println!("---");
                println!("ID:       {}", &note.id[..8]);
                println!("Paper:    {}", &note.paper_id[..8]);
                println!("Type:     {}", note.note_type.as_str());
                println!("Content:  {}", truncate(&note.content, 100));
            }
        }
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max]) }
}
