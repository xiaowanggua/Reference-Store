# Paper Search Skill

Search and browse papers in the Refstore library.

## When to use

User asks to find, search, look up, list, browse, or count papers.

## Search syntax (grep-like)

The `ref search` command supports advanced query syntax:

| Syntax | Example | Meaning |
|--------|---------|---------|
| Space-separated words | `deep learning` | AND — both words must appear |
| Quoted phrase | `"attention mechanism"` | Exact phrase match |
| Prefix wildcard | `transform*` | Matches transformer, transformers, etc. |
| Exclude with `-` | `deep -learning` | Has "deep" but NOT "learning" |
| Exclude with NOT | `deep NOT learning` | Same as above |
| OR operator | `transformer OR attention` | Either word |

### Examples

```bash
ref search "deep learning"              # papers with both "deep" AND "learning"
ref search '"attention mechanism"'      # exact phrase
ref search "transformer OR attention"   # either word
ref search "deep -learning"             # "deep" but NOT "learning"
ref search "transform*"                 # prefix wildcard
ref search "BERT"                       # single word = prefix match (BERT*)
```

### Search with filters

```bash
ref search "transformer" --tag NLP                    # only papers tagged NLP
ref search "transformer" --group survey               # only in "survey" group
ref search "neural" --in title                        # search only in titles
ref search "neural" --in abstract                     # search only in abstracts
ref search "deep learning" --tag NLP --page 2 --page-size 5
```

## List papers

```bash
ref list                                              # default: page 1, 10 per page, table format
ref list --page 2 --page-size 20                     # pagination
ref list --tag NLP                                    # filter by tag
ref list --group "survey"                             # filter by group
ref list --status read                                # only read papers
ref list --status unread                              # only unread papers
ref list --sort date                                  # sort by publish date
ref list --sort title                                 # sort by title
ref list --format compact                             # ID + title only
ref list --format json                                # JSON output for scripting
```

## Count papers

Use `ref count` when the user just wants to know how many papers match. Returns a single number — much cheaper than listing.

```bash
ref count                                             # total papers
ref count --tag NLP                                   # papers tagged NLP
ref count --status unread                             # unread papers
ref count --tag "Deep Learning" --group "thesis"     # combined filters
```

## View paper details

```bash
ref get <id>                                          # human-readable text (default)
ref get <id> --format json                            # JSON output
```

## Important

- Always use `--page-size 10` (or smaller) for lists and search. Never dump all papers at once.
- Use `ref count` first if the user just wants to know quantities.
- Short IDs (first 8 characters) work everywhere a full ID is expected.
- Present results in a readable format — the default text output is designed for both humans and AI.
- Use `--format json` only when you need to process results programmatically.
