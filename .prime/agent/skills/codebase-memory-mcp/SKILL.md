---

name: codebase-memory-mcp
description: "Use the codebase knowledge graph for structural code queries. Triggers on: explore the codebase, understand the architecture, what functions exist, show me the structure, who calls this function, what does X call, trace the call chain, find callers of, show dependencies, impact analysis, dead code, unused functions, high fan-out, refactor candidates, code quality audit, graph query syntax, Cypher query examples, edge types, how to use search_graph."

---

# Codebase Memory Instructions

The knowledge-graph MCP server (`codebase-memory-mcp`) is reached over stdio
from the IPython kernel. Import the skill once, then call its tools with
`await`:

```python
import codebase_memory_mcp

# 1. Verify the project is indexed
projects = await codebase_memory_mcp.list_projects()
print(projects)

# 2. Discover available tools (server is the source of truth for names/args)
for tool in await codebase_memory_mcp.list_tools():
    print(tool["name"], "-", tool["description"])

# 3. Search for a symbol pattern
results = await codebase_memory_mcp.search_graph(name_pattern=".*ProcessOrder.*", limit=10)
print(results)

# 4. Trace callers/callees
trace = await codebase_memory_mcp.trace_path(function_name="ProcessOrder", direction="inbound", depth=3)
print(trace)
```

Notes:
- Every tool is an `async` method — always `await`.
- Results are already-parsed Python (a `dict` for structured output, otherwise a
  string). No need to `json.loads` them.
- If a tool name is not a valid Python identifier, use the escape hatch:
  `await codebase_memory_mcp.call_tool("tool-name", {"arg": "value"})`.
- No login or credentials are required: the server is local and runs over
  stdio. If a call raises a binary-not-found error, set
  `CODEBASE_MEMORY_MCP_BIN` or add the binary to PATH.

# Quick Decision Matrix

| Question | Tool call |
|-------|--------|
Who calls X? | `await codebase_memory_mcp.trace_path(direction="inbound")`
What does X call? | `await codebase_memory_mcp.trace_path(direction="outbound")`
Full call context | `await codebase_memory_mcp.trace_path(direction="both")`
Find by name pattern | `await codebase_memory_mcp.search_graph(name_pattern="...")`
Dead code | `await codebase_memory_mcp.search_graph(max_degree=0, exclude_entry_points=true)`
Cross-service edges| `await codebase_memory_mcp.query_graph` with Cypher
Impact of local changes | `await codebase_memory_mcp.detect_changes()`
Risk-classified trace| `await codebase_memory_mcp.trace_path(risk_labels=true)`
Text search | `await codebase_memory_mcp.search_code(...)` or Grep

# Exploration Workflow

1. `await codebase_memory_mcp.list_projects()` — check if project is indexed
2. `await codebase_memory_mcp.get_graph_schema()` — understand node/edge types
3. `await codebase_memory_mcp.search_graph(label="Function", name_pattern=".*Pattern.*")` — find code
4. `await codebase_memory_mcp.get_code_snippet(qualified_name="project.path.FuncName")` — read source

# Tracing Workflow

1. `await codebase_memory_mcp.search_graph(name_pattern=".*FuncName.*")` — discover exact name
2. `await codebase_memory_mcp.trace_path(function_name="FuncName", direction="both", depth=3)` — trace
3. `await codebase_memory_mcp.detect_changes()` — map git diff to affected symbols

# Evidence Tiers

- Scout (Tier 1): fast positive lookup with few graph calls and targeted source checks. Treat results as provisional; never make absence, exhaustive, dead-code, or complete-impact claims.
- Verify (Tier 2, default): task-directed searches, relevant trace directions, exact snippets for material claims, and all relevant result pages.
- Auditor (Tier 3): bounded-scope full verification with a current graph generation, complete relevant pagination, both call directions and broader relationships when material, plus explicit unresolved limitations.
- Every tier: after candidate paths are known, call check_index_coverage once with every evidence path. For negative or exhaustive claims also include the relevant scopes. A clean result means no recorded gap, not proof of completeness. For partial, skipped, excluded, stale, pending, or unknown coverage, read/grep the reported ranges or scope before relying on the graph.

# Sessions and Subagents

- At session start or after compaction, call list_projects/index_status before structural exploration, then choose Scout, Verify, or Auditor for the task.
- Before delegating, query the graph and coverage in the parent. Pass the tier, exact project, generation/freshness, bounded scope, queries and pagination state, qualified symbols, paths, call-chain findings, coverage ranges/reasons, source fallback already performed, and unresolved questions to the child.
- Runtimes such as Hermes isolate child context: put those graph findings in the context argument to delegate_task; do not assume the child inherits MCP access or the parent's conversation.
- A child without MCP tools must not call or claim MCP access. It should work from the supplied evidence and use read/grep on exact source, especially every reported missed-coverage range.

# Quality Analysis

Dead code: `await codebase_memory_mcp.search_graph(max_degree=0, exclude_entry_points=true)`
High fan-out: `await codebase_memory_mcp.search_graph(min_degree=10, relationship="CALLS", direction="outbound")`
High fan-in: `await codebase_memory_mcp.search_graph(min_degree=10, relationship="CALLS", direction="inbound")`

# 15 MCP Tools (method calls on the module)

index_repository, index_status, list_projects, delete_project, search_graph, search_code, trace_path, detect_changes, query_graph, get_graph_schema, get_code_snippet, get_architecture, check_index_coverage, manage_adr, ingest_traces

# Edge Types

CALLS, HTTP_CALLS, ASYNC_CALLS, DATA_FLOWS, IMPORTS, DEFINES, DEFINES_METHOD, HANDLES, IMPLEMENTS, OVERRIDE, USAGE, CALL_REFERENCE, CONFIGURES, FILE_CHANGES_WITH, SIMILAR_TO, SEMANTICALLY_RELATED, CONTAINS_FILE, CONTAINS_FOLDER, CONTAINS_PACKAGE

# Cypher Examples (for query_graph)

```python
await codebase_memory_mcp.query_graph(query="MATCH (a)-[r:HTTP_CALLS]->(b) RETURN a.name, b.name, r.url_path, r.confidence LIMIT 20")
await codebase_memory_mcp.query_graph(query="MATCH (f:Function) WHERE f.name =~ '.*Handler.*' RETURN f.name, f.file_path")
await codebase_memory_mcp.query_graph(query="MATCH (a)-[r:CALLS]->(b) WHERE a.name = 'main' RETURN b.name")
```

# Gotchas
- search_graph(relationship="HTTP_CALLS") filters nodes by degree — use query_graph with Cypher to see actual edges.
- query_graph has a 100k row ceiling — add a Cypher LIMIT for broad queries or use search_graph pagination.
- trace_path needs exact names — use search_graph(name_pattern=...) first.
- direction="outbound" misses cross-service callers — use direction="both".
- search_graph results default to 50 per page — check has_more and use offset.
