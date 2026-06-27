# `src-tauri/` — OZ-POS desktop and mobile shell

The Tauri v2 binary that hosts the React/TypeScript front-end, wires
`oz-core` and `oz-hal` behind typed commands, and produces the installable
desktop bundle (and mobile archive, once mobile targets are added).

## Layout

```
src-tauri/
├── Cargo.toml              # oz-pos-app crate (binary + lib)
├── tauri.conf.json         # Tauri v2 app config
├── build.rs                # tauri_build::build() (icons, capabilities)
├── capabilities/
│   └── default.json        # ACL for the main window
└── src/
    ├── main.rs             # binary entry; calls lib::run()
    ├── lib.rs              # Builder, invoke_handler!, run()
    ├── error.rs            # AppError (typed, non_exhaustive)
    ├── state.rs            # AppState (DB, registry, app handle)
    └── commands/
        ├── mod.rs
        ├── health.rs       # ping, version
        ├── sales.rs        # start_sale, add_line, complete_sale
        └── hardware.rs     # open_cash_drawer, print_receipt
```

## Adding a new command

1. Create `src/commands/<feature>.rs` with `*Args` / `*Result` structs and
   a `#[tauri::command] async fn` taking `State<'_, AppState>`.
2. Add `pub mod <feature>;` to `src/commands/mod.rs`.
3. Add the command(s) to the `invoke_handler!` macro in `src/lib.rs`.
4. On the front-end, add a typed wrapper in `ui/src/api/pos.ts` and a
   hook in `ui/src/features/<feature>/`.

Full checklist in `.agents/skills/tauri-ipc/SKILL.md`.

## Icons (Tauri requirement)

`tauri-build` embeds the bundle icons listed in `tauri.conf.json` at
compile time. The scaffold ships **without** PNG/ICO files because those
are binary assets that should be generated, not hand-crafted.

Generate them once you have a source image (1024×1024 PNG works best):

```bash
cargo install tauri-cli --version "^2"
cargo tauri icon path/to/source.png
```

The command writes the full icon set to `src-tauri/icons/` and updates
`tauri.conf.json` if needed.

Until icons exist, `cargo build -p oz-pos-app` will fail at the
resource-embedding step. `cargo check` works because the build script
runs in a tolerant mode for non-bundled targets.

## Running locally

```bash
# Terminal 1 — Vite dev server for the React front-end
cd ui && npm run dev

# Terminal 2 — Tauri dev shell (loads http://localhost:1420)
cd src-tauri && cargo tauri dev
```

In production, `cargo tauri build` produces platform-specific bundles in
`src-tauri/target/release/bundle/`.

> last audited 28-06-26 by docs-auditor
