"""Codebase Memory integration: knowledge-graph MCP tools over stdio.

The codebase-memory-mcp server (DeusData/codebase-memory-mcp) speaks MCP over
stdio only — its HTTP daemon on :9749 is the graph UI, not an MCP endpoint.
This skill therefore overrides ``McpIntegration._open_session`` to spawn the
native binary via the MCP SDK's stdio client instead of the default
streamable-HTTP transport, so no bearer token or ``auth.json`` entry is needed.

Usage in the kernel:

    import codebase_memory_mcp
    projects = await codebase_memory_mcp.list_projects()
    results = await codebase_memory_mcp.search_graph(name_pattern=".*ProcessOrder.*")
"""

from __future__ import annotations

import os
import shutil
from contextlib import AsyncExitStack
from pathlib import Path

from rlm import McpIntegration

__all__ = ["CodebaseMemory", "codebase_memory_mcp"]


def _agent_dir() -> Path:
    """Resolve the Prime Agent config dir the same way the rest of the runtime does."""
    raw = (
        os.environ.get("PRIME_AGENT_CODING_AGENT_DIR")
        or os.environ.get("PI_CODING_AGENT_DIR")
        or str(Path.home() / ".prime" / "agent")
    )
    return Path(raw).expanduser().resolve()


def _settings_stdio_command() -> str | None:
    """Read a stdio ``command`` from the host settings mcpServers entry, if declared."""
    try:
        import json

        data = json.loads((_agent_dir() / "settings.json").read_text())
        servers = data.get("mcpServers") or {}
        entry = servers.get("codebase-memory-mcp")
        if isinstance(entry, dict) and entry.get("type") == "stdio":
            command = entry.get("command")
            if isinstance(command, str) and command.strip():
                command = command.strip()
                if os.path.isfile(command) or shutil.which(command):
                    return command
    except (OSError, ValueError):
        pass
    return None


def _stderr_log_path() -> Path:
    """Path where the MCP server's stderr is captured instead of ``sys.stderr``.

    mcp's ``stdio_client`` forwards ``errlog`` to the spawned subprocess, and the
    kernel's ``sys.stderr`` is an ipykernel stream without a ``fileno()``, which
    makes ``subprocess.Popen`` raise ``io.UnsupportedOperation`` before the
    server ever starts. A real file sidesteps that on every transport path.
    """
    log_dir = _agent_dir() / "logs"
    try:
        log_dir.mkdir(parents=True, exist_ok=True)
    except OSError:
        pass
    return log_dir / "codebase-memory-mcp-stderr.log"


def _known_install_candidates() -> list[str]:
    """Well-known install locations for the native binary (checked after PATH)."""
    candidates: list[str] = []
    local_app_data = os.environ.get("LOCALAPPDATA", "")
    if local_app_data:
        # Versioned install root first (newest version wins) — the daemon it must
        # talk to runs the same version; an older `Programs` copy would refuse to
        # start while a newer daemon is active.
        versioned_root = os.path.join(local_app_data, "codebase-memory-mcp")
        try:
            for name in sorted(os.listdir(versioned_root), reverse=True):
                candidates.append(
                    os.path.join(versioned_root, name, "codebase-memory-mcp.exe")
                )
        except OSError:
            pass
        candidates.append(
            os.path.join(
                local_app_data,
                "Programs",
                "codebase-memory-mcp",
                "codebase-memory-mcp.exe",
            )
        )
    candidates.append(os.path.join(str(Path.home()), ".local", "bin", "codebase-memory-mcp"))
    candidates.append("/usr/local/bin/codebase-memory-mcp")
    return candidates


def _resolve_binary() -> str:
    """Locate the codebase-memory-mcp executable.

    Resolution order: the ``CODEBASE_MEMORY_MCP_BIN`` env var, the stdio
    ``command`` from the host's ``mcpServers`` setting, the system PATH, then
    known install locations. Raises a descriptive error when not found.
    """
    explicit = os.environ.get("CODEBASE_MEMORY_MCP_BIN", "").strip()
    if explicit:
        return explicit
    settings_command = _settings_stdio_command()
    if settings_command:
        return settings_command
    found = shutil.which("codebase-memory-mcp")
    if found:
        return found
    for candidate in _known_install_candidates():
        if os.path.isfile(candidate):
            return candidate
    raise RuntimeError(
        "codebase-memory-mcp binary not found: set CODEBASE_MEMORY_MCP_BIN, declare a "
        "stdio `command` under mcpServers in the agent settings, or add it to PATH"
    )


class CodebaseMemory(McpIntegration):
    """Knowledge-graph MCP client backed by the local codebase-memory-mcp stdio server."""

    server = "codebase-memory-mcp"

    async def _open_session(self, stack: AsyncExitStack):
        from mcp import ClientSession, StdioServerParameters, stdio_client

        # Route the server's stderr to a real log file: mcp forwards it to the
        # spawned subprocess, and inside the IPython kernel sys.stderr has no
        # fileno(), so Popen would die with io.UnsupportedOperation.
        errlog = open(_stderr_log_path(), "a", encoding="utf-8", errors="replace")
        stack.callback(errlog.close)

        params = StdioServerParameters(command=_resolve_binary())
        read, write = await stack.enter_async_context(stdio_client(params, errlog=errlog))
        session = await stack.enter_async_context(ClientSession(read, write))
        await session.initialize()
        return session


codebase_memory_mcp = CodebaseMemory()


# Names the kernel bootstrap probes to decide if a module is a callable skill.
# Don't forward them, or `getattr(module, "run")` returns an MCP tool stub and the
# module gets wrapped as callable, breaking `await codebase_memory_mcp.<tool>()` dispatch.
_RESERVED = {"run", "__wrapped__", "__call__"}


def __getattr__(name: str):
    # Forward bare module-level access (e.g. codebase_memory_mcp.search_graph) to the
    # instance, so `import codebase_memory_mcp; await codebase_memory_mcp.search_graph(...)`
    # works without an extra `.codebase_memory_mcp` step.
    if name.startswith("_") or name in _RESERVED:
        raise AttributeError(name)
    return getattr(codebase_memory_mcp, name)
