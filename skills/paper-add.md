# Paper Add Skill

Add a paper to the Refstore library.

## When to use

User asks to save, add, store, or record a paper.

## How to add

### By DOI (preferred — auto-fetches metadata from CrossRef)

The tool automatically fetches title, authors, abstract, venue, and publish date from the CrossRef API. Any field explicitly provided via flags overrides the auto-fetched value.

```bash
ref add --doi "10.1038/s41586-020-2649-2"
```

### By arXiv ID (auto-fetches metadata from arXiv API)

Supports various formats: `2301.07041`, `arXiv:2301.07041`, `https://arxiv.org/abs/2301.07041`. The tool normalizes them automatically.

```bash
ref add --arxiv 2301.07041
```

### Manual entry (when no DOI/arXiv available)

`--title` is required for manual entry. Other fields are optional.

```bash
ref add --title "Paper Title" --authors "Author One,Author Two" --url "https://..." --abstract-text "..." --venue "NeurIPS 2024" --date 2024-01-15
```

### Override auto-fetched fields

Explicitly provided flags override auto-fetched values:

```bash
ref add --doi "10.1038/..." --title "Custom Title" --authors "Alice,Bob"
```

### Bulk import from BibTeX file

```bash
ref import papers.bib
```

### Restore from JSON backup

```bash
ref import backup.json
```

## Duplicate detection

Automatic dedup runs on every add (unless `--force`). Priority: **arXiv ID > DOI > title** (case-insensitive).

- If a duplicate is found, the tool returns the existing paper info and does NOT insert a new entry.
- Use `--force` to bypass dedup and always insert:

```bash
ref add --doi "10.1038/..." --force
```

## After adding

Tag the paper immediately:

```bash
ref tag add <id> NLP
ref tag add <id> BERT --parent NLP --alias "Bidirectional Encoder"
```

Add it to a group:

```bash
ref group assign <id> "thesis-papers"
```

## Important

- When DOI/arXiv fetch fails, the tool prints a warning and falls back to manual entry. In that case, `--title` is required.
- Use `ref get <id>` to verify the paper was added correctly.
- Short IDs (first 8 characters) work for all subsequent commands.
