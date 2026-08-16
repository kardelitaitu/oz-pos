---
name: codebase-memory-mcp
description: "Use the codebase knowledge graph for structural code queries. Triggers on: explore the codebase, understand the architecture, what functions exist, show me the structure, who calls this function, what does X call, trace the call chain, find callers of, show dependencies, impact analysis, dead code, unused functions, high fan-out, refactor candidates, code quality audit, graph query syntax, Cypher query examples, edge types, how to use search_graph."
---

# Codebase Memory MCP

Local knowledge-graph server over stdio — no credentials, no HTTP. Import the
module once, then `await` any tool directly:

```python
import codebase_memory_mcp
```

## Quick Start

```python
import codebase_memory_mcp

# 1. Verify project is indexed
projects = await codebase_memory_mcp.list_projects()

# 2. Search for a symbol
results = await codebase_memory_mcp.search_graph(project="oz-pos", name_pattern=".*Money.*", limit=5)

# 3. Read its source
source = await codebase_memory_mcp.get_code_snippet(project="oz-pos", qualified_name="oz-pos.crates.oz-core.src.money.Money")

# 4. Trace callers
trace = await codebase_memory_mcp.trace_path(project="oz-pos", function_name="Money", direction="inbound", depth=3)
```

> **Every tool is `async`** — always `await`. Results are parsed Python
> (dict or string). No `json.loads` needed.

---

## Tool Reference

### Project Management

| Tool | Purpose | Key Params |
|------|---------|------------|
| `list_projects` | List all indexed projects | — |
| `index_status` | Node/edge counts + coverage report | `project` |
| `index_repository` | (Re)index a repository | `repo_path`, `name`, `mode` |
| `delete_project` | Remove a project from the index | `project` |

### Search & Discovery

| Tool | Purpose | Key Params |
|------|---------|------------|
| `search_graph` | Find symbols by name, query, or semantic similarity | `project`, `name_pattern` / `query` / `semantic_query`, `label`, `limit`, `offset` |
| `search_code` | Grep + graph enrichment (dedup into functions) | `project`, `pattern`, `limit`, `file_pattern`, `path_filter`, `mode` |
| `get_code_snippet` | Read source for a symbol | `project`, `qualified_name` |

### Tracing & Impact

| Tool | Purpose | Key Params |
|------|---------|------------|
| `trace_path` | Callers/callees, data flow, cross-service | `project`, `function_name`, `direction`, `depth`, `mode` |
| `detect_changes` | Blast radius of git diff | `project`, `direction` |
| `check_index_coverage` | Verify file is indexed (before claims) | `project`, `paths` / `scopes` |

### Architecture & Analysis

| Tool | Purpose | Key Params |
|------|---------|------------|
| `get_graph_schema` | Node labels + edge types | `project` |
| `get_architecture` | High-level overview or deep analysis | `project`, `aspects` |
| `query_graph` | Cypher queries for complex patterns | `project`, `query`, `graph` |

### Advanced

| Tool | Purpose | Key Params |
|------|---------|------------|
| `manage_adr` | Architecture Decision Records | `project`, `mode` |
| `ingest_traces` | Enhance graph with runtime traces | `project` |

---

## Search Modes

`search_graph` supports three independent modes (can combine):

| Mode | When to use | Example |
|------|-------------|---------|
| `name_pattern` | Regex on symbol names | `".*ProcessOrder.*"` |
| `query` | BM25 full-text (natural language) | `"update settings"` |
| `semantic_query` | Vector cosine (vocabulary bridging) | `["send", "publish"]` |

**Pagination**: results cap at `limit` (default 50). Check `has_more` in
response; re-call with `offset=offset+limit` until false. Narrow first via
`label`, `file_pattern`, `min_degree`.

---

## Trace Modes

| `direction` | What it traces |
|-------------|----------------|
| `inbound` | Who calls this function? |
| `outbound` | What does this function call? |
| `both` | Full call context |

| `mode` | What it traces |
|--------|----------------|
| `calls` (default) | Callers/callees |
| `data_flow` | Value propagation with args at each hop |
| `cross_service` | Through HTTP/async Route nodes |

**Pagination**: `truncated: true` + `next` cursor — pass `next` back.

---

## Architecture Aspects

`get_architecture` accepts `aspects` list:

- `overview` — counts, languages, packages, entry_points
- `structure` — module organization
- `dependencies` — external deps
- `routes` — HTTP/async endpoints
- `hotspots` — high-complexity functions
- `boundaries` — module boundaries
- `layers` — architectural layers
- `clusters` — Leiden community detection (real seams)
- `file_tree` — directory structure
- `all` — everything above

---

## Graph Schema

### Node Labels (19)

`Function`, `Variable`, `Section`, `Field`, `Method`, `File`, `Module`,
`Struct`, `Interface`, `Folder`, `Class`, `Type`, `Route`, `Enum`, `EnvVar`,
`Package`, `Decorator`, `Branch`, `Project`

### Edge Types (24)

`CALLS`, `HTTP_CALLS`, `ASYNC_CALLS`, `DATA_FLOWS`, `IMPORTS`, `DEFINES`,
`DEFINES_METHOD`, `HANDLES`, `IMPLEMENTS`, `OVERRIDE`, `USAGE`,
`CALL_REFERENCE`, `CONFIGURES`, `FILE_CHANGES_WITH`, `SIMILAR_TO`,
`SEMANTICALLY_RELATED`, `CONTAINS_FILE`, `CONTAINS_FOLDER`, `CONTAINS_PACKAGE`,
`DEPENDS_ON`, `RAISES`, `THROWS`, `INHERITS`, `TESTS`, `TESTS_FILE`,
`HAS_BRANCH`

### Complexity Properties (on Function/Method nodes)

`complexity` (cyclomatic), `cognitive`, `loop_count`, `loop_depth`,
`transitive_loop_depth`, `linear_scan_in_loop`, `alloc_in_loop`,
`recursion_in_loop`, `unguarded_recursion`, `recursive`, `param_count`,
`max_access_depth`

---

## Cypher Examples

```python
# Find HTTP routes
await codebase_memory_mcp.query_graph(project="oz-pos",
    query="MATCH (a)-[r:HTTP_CALLS]->(b) RETURN a.name, b.name, r.url_path LIMIT 20")

# Find all handlers
await codebase_memory_mcp.query_graph(project="oz-pos",
    query="MATCH (f:Function) WHERE f.name =~ ".*Handler.*" RETURN f.name, f.file_path")

# Find hot-path candidates (high complexity + scan in loop)
await codebase_memory_mcp.query_graph(project="oz-pos",
    query="MATCH (f:Function) WHERE f.transitive_loop_depth >= 3 OR f.linear_scan_in_loop >= 1 RETURN f.qualified_name, f.transitive_loop_depth, f.linear_scan_in_loop ORDER BY f.transitive_loop_depth DESC")

# Query missed graph (files not fully indexed)
await codebase_memory_mcp.query_graph(project="oz-pos", graph="missed",
    query="MATCH (f:File) WHERE f.kind = \"parse_partial\" RETURN f.file_path, f.detail")
```

> **100k row ceiling** — always add `LIMIT` to Cypher or use `search_graph`
> pagination.

---

## Quality Analysis Queries

```python
# Dead code (no callers, not entry points)
await codebase_memory_mcp.search_graph(project="oz-pos", max_degree=0, exclude_entry_points=True)

# High fan-out (calls many functions)
await codebase_memory_mcp.search_graph(project="oz-pos", min_degree=10, relationship="CALLS", direction="outbound")

# High fan-in (called by many)
await codebase_memory_mcp.search_graph(project="oz-pos", min_degree=10, relationship="CALLS", direction="inbound")
```

---

## Decision Matrix

| Question | Tool |
|----------|------|
| Who calls X? | `trace_path(direction="inbound")` |
| What does X call? | `trace_path(direction="outbound")` |
| Full call context | `trace_path(direction="both")` |
| Find by name pattern | `search_graph(name_pattern="...")` |
| Find by description | `search_graph(query="...")` |
| Find by concept | `search_graph(semantic_query=[...])` |
| Read source code | `get_code_snippet(qualified_name="...")` |
| Text search (grep) | `search_code(pattern="...")` |
| Dead code | `search_graph(max_degree=0, exclude_entry_points=True)` |
| Impact of changes | `detect_changes()` |
| Architecture overview | `get_architecture(aspects=["overview"])` |
| Module clusters | `get_architecture(aspects=["clusters"])` |
| Hot-path analysis | `query_graph(query="...transitive_loop_depth...")` |
| Coverage check | `check_index_coverage(paths=[...])` |
| Cross-service edges | `query_graph(query="...CROSS_HTTP_CALLS...")` |

---

## Workflows

### Explore unfamiliar code

1. `list_projects()` → verify indexed
2. `get_graph_schema()` → understand structure
3. `get_architecture(aspects=["overview", "clusters"])` → high-level map
4. `search_graph(label="Function", name_pattern=".*Pattern.*")` → find code
5. `get_code_snippet(qualified_name="...")` → read source

### Trace a call chain

1. `search_graph(name_pattern=".*FuncName.*")` → exact qualified name
2. `trace_path(function_name="FuncName", direction="both", depth=3)` → trace
3. `get_code_snippet(qualified_name="...")` → verify source

### Impact analysis before refactor

1. `detect_changes()` → blast radius of current diff
2. `trace_path(direction="inbound", depth=5)` → full caller tree
3. `check_index_coverage(paths=[...])` → verify all paths indexed
4. `search_code(pattern="...")` → text fallback for unindexed ranges

### Code quality audit

1. `search_graph(max_degree=0, exclude_entry_points=True)` → dead code
2. `search_graph(min_degree=10, relationship="CALLS", direction="outbound")` → high fan-out
3. `query_graph(query="...complexity >= 10...")` → high complexity
4. `get_architecture(aspects=["hotspots"])` → bottleneck summary

---

## Evidence Tiers

| Tier | Scope | When to use |
|------|-------|-------------|
| **Scout** | Fast positive lookup, few graph calls | Quick exploration, provisional findings |
| **Verify** | Task-directed searches, all pages, snippets | Default for most tasks |
| **Auditor** | Full verification, both directions, coverage check | Before major refactors, exhaustive claims |

**Every tier**: after candidate paths are known, call `check_index_coverage`
with every evidence path. For negative/exhaustive claims, also include scopes.
A clean result = no recorded gap, not proof of completeness.

---

## Sessions & Subagents

- **Session start**: call `list_projects()` / `index_status()` before exploration
- **Before delegating**: query graph in parent; pass tier, project, scope,
  symbols, paths, findings, and coverage to child
- **Child without MCP**: must not claim MCP access; work from supplied evidence

---

## Gotchas

1. **`search_graph(relationship=...)` filters nodes by degree** — use
   `query_graph` with Cypher to see actual edges
2. **`query_graph` 100k row ceiling** — add `LIMIT` or use `search_graph`
   pagination
3. **`trace_path` needs exact names** — use `search_graph(name_pattern=...)` first
4. **`direction="outbound"` misses cross-service** — use `direction="both"`
5. **`search_graph` defaults to 50/page** — check `has_more`, use `offset`
6. **`get_code_snippet` is a read tool** — call `search_graph` first to get
   the exact `qualified_name`
7. **`search_code` truncation** — check `total_grep_matches` vs `limit`;
   narrow with `file_pattern` / `path_filter`
8. **Coverage is best-effort** — `indexed_no_recorded_gap` ≠ completeness
   guarantee; grep flagged ranges before relying on graph
9. **Binary resolution** — set `CODEBASE_MEMORY_MCP_BIN` if not on PATH;
   version must match running MCP server
