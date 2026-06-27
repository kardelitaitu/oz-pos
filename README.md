# OZ-POS

> *"Pay no attention to the man behind the curtain."* — The Wizard of Oz

**OZ-POS** is a magical, modular Point-of-Sale software framework built with Rust and Tauri v2. Like the wizard behind the curtain, it silently powers fast, reliable checkout experiences — while merchants and developers only see effortless simplicity.

The name **OZ** embodies four pillars:

| | Pillar | Meaning |
|-|--------|---------|
| 🧙 | **Magical** | Complex operations feel effortless — barcodes, payments, encryption, sync |
| 🧵 | **Small Core** | Lean `oz-core` crate; every feature is a composable module |
| ♾️ | **Limitless** | Scales from a single warung to a nationwide chain — no rewrite |
| 📈 | **Scalable** | Horizontal scaling, cloud sync, multi-store — built in from day one |

---

## What is OZ-POS?

A high-performance POS framework for retail stores, restaurants, and any merchant environment. It runs on Windows PCs, Linux PCs, Android tablets, and iPads with full barcode scanner support, offline-first operation, and optional cloud sync.

---

## Key Features

- **Barcode Support** – USB, Bluetooth, and serial barcode scanners via a unified HAL
- **Multi-Currency** – Integer-based money handling with ISO-4217 currency codes
- **Offline-First** – SQLite on-device storage; cloud sync is optional
- **Extensible** – Embedded Lua scripting for custom business rules and promotions
- **Cross-Platform** – Windows, Linux, Android, iPad from a single codebase
- **Cloud Optional** – On-features cloud DB, analytics, and backup add-ons

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Core Engine | Rust |
| UI | Tauri v2 + React + TypeScript |
| Scripting | Lua (via `rlua`) |
| Local DB | SQLite (`rusqlite`) |
| Cloud DB | PostgreSQL / CockroachDB (optional) |
| Cache | Redis (optional) |
| Build | Cargo + Vite + GitHub Actions |

---

## Documentation

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Directory layout and module responsibilities |
| [WHITEPAPER.md](./WHITEPAPER.md) | Design rationale, tech choices, database strategy |
| [ROADMAP.md](./ROADMAP.md) | Phased milestones and planned features |
| [CONTRIBUTING.md](./CONTRIBUTING.md) | Contribution workflow, branch policy, PR checklist |
| [docs/QUICKSTART.md](./docs/QUICKSTART.md) | First-time local setup and build |

---

## Getting Started

### Prerequisites

- Rust stable toolchain (`rustup install stable`)
- Node.js >= 18
- Tauri v2 prerequisites ([see Tauri docs](https://tauri.app/v2/guides/))

### Build & Run

```bash
# 1. Clone the repository
git clone https://github.com/kardelitaitu/oz-pos.git
cd oz-pos

# 2. Build Rust workspace
cargo build --workspace

# 3. Install front-end dependencies
cd ui && npm install

# 4. Run in development mode
npm run dev   # launches Tauri dev window
```

### Run on Android / iPad

```bash
# Android (requires Android SDK)
cargo tauri android dev

# iOS/iPadOS (requires Xcode on macOS)
cargo tauri ios dev
```

---

## Project Structure

```
oz-pos/
├─ src/          # Rust crates (core, hal, lua, security, logging, payment, reporting, perf, cli)
├─ ui/           # React + TypeScript front-end
├─ migrations/   # SQL migration files
├─ docs/         # Additional documentation
└─ .github/      # CI/CD workflows
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full directory tree.

---

## License

MIT License. See `LICENSE` for details.

---

*Built with Rust, Tauri v2, and a love for reliable software.*
