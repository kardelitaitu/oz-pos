---
title: Docs Styling Guide
description: How to use callouts, links, tables, and code when writing documentation.
category: reference
order: 3
updated: "2026-08-15"
---

## Callouts

Callouts highlight important information. Write a blockquote whose first line
starts with a bold label — the label picks the color:

> **Note:** General information worth knowing. The default callout style.

> **Info:** Background context or additional details.

> **Tip:** A shortcut, best practice, or recommended approach.

> **Warning:** Something to be careful about — it may not do what you expect.

> **Danger:** An action that can cause data loss or break an installation.

Any text after the bold label is the callout body:

> **Warning:** Always back up your database before running a migration.

## Links

Relative links between docs pages use the page slug:

```
See the [cloud sync](../cloud-sync/) guide.
```

Renders as: See the [cloud sync](../cloud-sync/) guide.

## Tables

Pipe tables render with a bordered style:

| Feature        | Included |
| -------------- | -------- |
| Cloud sync     | ✓        |
| QRIS payments  | ✓        |
| Lua scripting  | ✓        |

## Code

Inline code uses backticks, e.g. `Money::from_minor(1000)`. Fenced blocks
render in a bordered, scrollable box:

```rust
let total = cart.total();
let due = total - discount;
```
