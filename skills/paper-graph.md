# Paper Graph Skill

Query and visualize citation relationships between papers.

## When to use

User asks about paper relationships, citations, what paper A cites, related papers, or wants a citation graph.

## Add a citation

```bash
ref cite add <from-id> <to-id> --relation cites --strength 4
ref cite add <from-id> <to-id> --relation extends --strength 3 --note "Section 3 extends this method"
```

### Relation types

| Type | Meaning |
|------|---------|
| `cites` | Paper A cites Paper B (default) |
| `related` | Papers are related in topic or approach |
| `contrasts` | Papers present contrasting methods or findings |
| `extends` | Paper A extends Paper B's work |
| `improves` | Paper A improves upon Paper B |

### Strength

Range: 1 (weak) to 5 (strong), default 3. Represents how significant the relationship is.

## List citations

Show all direct citations for a paper (both outgoing and incoming):

```bash
ref cite list <id>
```

## View citation graph

Recursively traverse citation relationships up to a given depth:

```bash
ref cite graph <id> --depth 1        # direct relations only
ref cite graph <id> --depth 2        # one hop further (default)
ref cite graph <id> --depth 3        # two hops
```

### Mermaid output

Generate a Mermaid diagram for rendering in Markdown (GitHub, Notion, etc.):

```bash
ref cite graph <id> --depth 3 --format mermaid
```

## Export graph

Export the citation graph for a specific paper:

```bash
ref export --format mermaid --graph <id>
```

## Important

- The graph is bidirectional — `ref cite list <id>` shows both papers it cites and papers that cite it.
- Use `--depth 1` for a quick overview, `--depth 2-3` for broader context.
- Mermaid format can be pasted directly into Markdown documents with ```mermaid code blocks.
- When presenting graph results, describe the relationships in natural language rather than just listing edges.
- Use `--note` when adding citations to record why the relationship exists — it appears in graph output.
