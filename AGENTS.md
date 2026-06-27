# Agents Configuration

## Global Rules

- Maintain documentation integrity. Preserve all existing comments and docstrings unless explicitly modified.

## Project Specific Rules

- Follow the POS software framework conventions.
- Ensure all code follows the project's coding standards.

### Rust Standards
- Format all Rust code with `rustfmt` before committing.
- Run `cargo clippy -- -D warnings` and resolve all warnings.
- Every public function, struct, and trait must have a doc comment (`///`).
- Prefer `thiserror` for error types and `anyhow` for application-level error propagation.
- Store all monetary values as integer minor units (`i64`) using the `Money` struct; never use `f32`/`f64` for currency.
- Use `rusqlite` with transactions for all database writes; never write outside a transaction.

### Tauri / UI Standards
- Tauri commands must be defined in `src-tauri/src/commands/` and registered in `main.rs`.
- Front-end API calls go through `ui/src/api/pos.ts`; do not call `invoke` directly in components.
- All React components must have ARIA labels and pass `eslint-plugin-jsx-a11y` checks.
- Use `@fluent/react` for all user-visible strings; no hardcoded English strings in JSX.

### Testing Standards
- Every new Rust module must include a `#[cfg(test)]` block with at least one unit test.
- HAL drivers must have a mock implementation in `hal/src/drivers/mock.rs` for testing.
- Front-end components must have a corresponding test in `ui/src/__tests__/`.

### Git & Branch Policy
- Branch naming: `feat/<name>`, `fix/<name>`, `docs/<name>`, `chore/<name>`.
- All PRs must pass the CI pipeline (lint, test, build) before merging.
- Never commit secrets, `.env` files, or SQLite database files.

> [!NOTE]
> This file serves as the central place to define agents, rules, and customization for the POS framework.
