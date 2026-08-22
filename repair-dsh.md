# Repair Plan — DSH `reasoning_content must be passed back` 400 error

**Status:** Applied ✅
**Date:** 2026-08-22
**Applied:** 2026-08-22 21:33–21:34 (local)
**Backups:**
- `index.js.bak-20260822-213350`
- `openai-completions.js.bak-20260822-213350`
**Affects:** DeepSeek Harness Desktop (DSH) agent runtime — NOT the oz-pos repo code.

---

## 1. Problem

Every so often a turn fails with:

```
400: {"message":"The `reasoning_content` in the thinking mode must be passed back to the API.",
     "type":"invalid_request_error","param":"","code":"invalid_request_error"}
```

It is not a bug in `oz-pos`. It comes from the DeepSeek chat-completions API when the
request contains an **assistant message that lacks `reasoning_content`** while the
request is in thinking mode. DeepSeek requires the field to be **present** on every
assistant message in a thinking-mode conversation — including turns that produced
no reasoning text (e.g. tool-call-only turns).

---

## 2. Root-cause chain (evidence-backed from the DSH install)

Install root: `C:\Program Files\DSH Desktop\resources\app.asar.unpacked\node_modules`

### 2a. The DeepSeek adapter serializer omits the field

`@deepseek-ai/dsh-llm-deepseek/lib/index.js:107-124` (`serializeAssistant`):

```js
function serializeAssistant(message) {
    const text = flattenText(message.content);
    const reasoning = message.content
        .filter((block) => block.type === "reasoning")
        .map((block) => block.text).join("");
    const toolCalls = message.content
        .filter((block) => block.type === "tool-call")
        .map((block) => ({ id: block.id, type: "function",
            function: { name: block.name, arguments: block.arguments } }));
    return {
        role: "assistant",
        content: text,
        ...reasoning.length > 0 ? { reasoning_content: reasoning } : {},   // ← OMITTED when empty
        ...toolCalls.length > 0 ? { tool_calls: toolCalls } : {}
    };
}
```

Line 121: when the assistant message has no `reasoning` block, `reasoning_content`
is **omitted entirely**. This is the primary defect.

This serializer is the one actually used by the DeepSeek adapter:
- `serializeMessages` (line 133) calls `serializeAssistant`
- used at lines 263 (`serializeMessages(options.messages)`) and 294 (`serializeMessagesWithImages`)

### 2b. Failure trigger

A request history that contains an assistant message with `tool_calls` but **no**
`reasoning` block triggers the 400. Sources of such messages:

1. **Tool-call-only turns** — DeepSeek sometimes streams a turn that only carries
   `delta.tool_calls` (no `reasoning_content` in that turn's deltas). The stored
   message keeps only the `tool-call` block.
2. **Compaction / checkpoint rebuilds** — after a session checkpoint (like the one
   that condensed a long conversation), older assistant messages may be rebuilt
   with text + tool calls but without their reasoning blocks.
3. **Foreign/provider replay** — assistant messages injected from another model or
   path that never carried reasoning.

### 2c. Secondary defect in the pi-ai fallback path

`@earendil-works/pi-ai/dist/api/openai-completions.js:924-928` (used by the pi-ai
adapter, not the DeepSeek adapter):

```js
if (compat.requiresReasoningContentOnAssistantMessages &&   // = isDeepSeek (line 1155)
    model.reasoning &&
    assistantMsg.reasoning_content === undefined) {
    assistantMsg.reasoning_content = "";                     // ← fills EMPTY STRING
}
```

DeepSeek **rejects `""`** too — it wants the field absent-or-real, not an empty
placeholder. So this fallback is also wrong for DeepSeek.

### 2d. What is NOT the cause (ruled out)

- Session persistence (`dsh-session`, `dsh-session-persistence-jsonl`) stores
  `reasoning` blocks correctly (`reasoning-chunks` → `reasoning-delta` → blocks).
- The harness's native block type is `"reasoning"` and the serializer filters on
  `"reasoning"` — consistent, no type mismatch.
- Compaction replaces old messages with a *user-role summary* (no reasoning
  needed) — not the direct cause, but rebuilt/trimmed messages can be.

---

## 3. Proposed fix

### Primary: patch `serializeAssistant` in the DeepSeek adapter

`@deepseek-ai/dsh-llm-deepseek/lib/index.js` lines 118-123 — emit the field
present (empty string) **only on tool-call-only assistant turns**; keep the
existing behavior otherwise:

```js
return {
    role: "assistant",
    content: text,
    // DeepSeek thinking mode requires reasoning_content on EVERY assistant
    // message. A turn that streamed reasoning echoes it; a tool-call-only
    // turn that streamed none still needs the field present (DeepSeek
    // rejects a missing field, and rejects a bare "" on non-tool turns too,
    // so scope the empty fill to tool-call-only messages).
    ...(reasoning.length > 0
        ? { reasoning_content: reasoning }
        : toolCalls.length > 0
            ? { reasoning_content: "" }
            : {}),
    ...toolCalls.length > 0 ? { tool_calls: toolCalls } : {}
};
```

### Secondary (defensive): pi-ai fallback

`@earendil-works/pi-ai/dist/api/openai-completions.js:924-928` — do not fill `""`
for DeepSeek; only fill for messages that carry `tool_calls` (same rule):

```js
if (compat.requiresReasoningContentOnAssistantMessages &&
    model.reasoning &&
    assistantMsg.reasoning_content === undefined &&
    assistantMsg.tool_calls) {          // only tool-call-only turns
    assistantMsg.reasoning_content = "";
}
```

---

## 4. Application steps (when we do it)

> The install dir is NOT writable from a normal shell:
> `C:\Program Files\DSH Desktop\resources\app.asar.unpacked\node_modules\...`
> Access denied on write — needs an elevated process (Run as Administrator).

1. **Quit DSH Desktop** (5 processes observed: `DSH Desktop`).
2. **Backup originals** (elevated):
   - `...\dsh-llm-deepseek\lib\index.js` → `index.js.bak-<timestamp>`
   - `...\@earendil-works\pi-ai\dist\api\openai-completions.js` → `.bak-<timestamp>`
   - A working (non-elevated) copy already exists at `%TEMP%\dsh-llm-deepseek-index.js.orig`.
3. **Apply patch #1** to `dsh-llm-deepseek/lib/index.js` (`serializeAssistant`).
4. **Apply patch #2** to `pi-ai/dist/api/openai-completions.js` (defensive).
5. **Sanity check** the edited files parse as JS:
   - `node --check <file>` (Node is available in the DSH node_modules runtime).
6. **Restart DSH Desktop.**
7. **Verify**: run a normal turn, then a tool-call-heavy turn (multi-step tool use),
   confirm no 400. Then reproduce the checkpoint scenario (long session + compact)
   and confirm it stays green.

---

## 5. Rollback

- Restore the `.bak-<timestamp>` files (elevated).
- Or `pnpm`-reinstall / reinstall the app (asar unpacked gets restored).

---

## 6. Alternative (cleaner) fix — worth checking first

Before patching the install, check whether a **newer `dsh-llm-deepseek` /
`pi-ai`** exists upstream (npm / GitHub). The empty-fill bug looks like a known
compat regression; an upgrade may fix it properly. Prefer upgrading over
hand-patching if a fixed release is available.

Also consider: the profile overlay `~/.dsh/profiles/desktop/cordis.patch.yml`
is the **sanctioned override mechanism** ("Edit configuration patch" in the UI).
If the loader supports patching node_modules bundles through it, that is the
cleanest non-elevated path — investigate before touching Program Files.

---

## 7. Definition of done

- [ ] No `reasoning_content` 400 across: normal turns, tool-heavy turns,
      long-session compaction, session resume, subagent forks.
- [x] Originals backed up.
- [x] Change recorded here.
- [ ] Rollback path verified.
