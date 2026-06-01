# Paper Notes Skill

Manage notes and tags on papers.

## When to use

User asks to add notes, write thoughts, tag papers, categorize, or search notes.

## Tags (describe paper attributes)

Tags describe what a paper is about (NLP, Transformer, survey). They are auto-created on first use.

### Add/remove tags

```bash
ref tag add <id> "Transformer"
ref tag remove <id> "CNN"
```

### Hierarchical tags

Use `--parent` to nest tags under a parent category:

```bash
ref tag add <id> BERT --parent NLP
ref tag add <id> GPT --parent NLP
```

### Tag aliases

Use `--alias` to add a synonym for a tag:

```bash
ref tag add <id> LLM --alias "Large Language Model"
```

### List and delete tags

```bash
ref tag list                  # show all tags in tree structure
ref tag delete "obsolete"     # delete a tag entirely
```

## Notes (Markdown annotations)

### Note types

| Type | When to use |
|------|-------------|
| `summary` | Summarize the paper's main contribution |
| `method` | Describe methodology or approach |
| `result` | Record key experimental results or findings |
| `thought` | Personal thoughts, ideas, or connections to other work |
| `general` | General-purpose notes (default) |

### Add a note

```bash
ref note add <id> --content "Key insight: self-attention replaces recurrence" --note-type summary
ref note add <id> --content "The 3-layer transformer achieved 92.4% accuracy" --note-type result
ref note add <id> --content "Could combine this with contrastive learning" --note-type thought
```

### List notes for a paper

```bash
ref note list <id>
```

### Search notes by keyword

Searches across all papers' notes. Supports pagination:

```bash
ref note search "self-attention" --page 1 --page-size 10
ref note search "transformer" --page 2 --page-size 5
```

### Update and delete

```bash
ref note update <note-id> --content "Updated text"
ref note update <note-id> --note-type thought
ref note delete <note-id>
```

## Groups (personal collections)

Groups organize papers by your purpose (thesis, project-A, reading-list). Different from tags.

```bash
ref group add "thesis-papers" --description "Papers for my thesis"
ref group assign <id> "thesis-papers"
ref group list
ref group unassign <id> "thesis-papers"
```

## Important

- Note content supports Markdown formatting.
- Use `ref note search` to find notes by keyword across all papers.
- Always use pagination (`--page`, `--page-size`) for note search results.
- Choose the appropriate note type — it helps when reviewing later.
- Tags = paper attributes (what it's about). Groups = personal collections (why you saved it).
