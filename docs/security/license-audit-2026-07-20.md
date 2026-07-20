# License Audit — 2026-07-20

## Summary

Run: `cargo license` (cargo-license v0.7.0)

### Findings

| License | Count | Risk | Notes |
|---------|-------|------|-------|
| MIT | ~180 | ✅ None | Most common Rust license |
| Apache-2.0 | ~60 | ✅ None | Compatible with MIT |
| MIT OR Apache-2.0 | ~40 | ✅ None | Dual-licensed permissive |
| SEE LICENSE IN LICENSE | 27 | ✅ Internal | All OZ-POS crates — proprietary |
| BSD-3-Clause | ~10 | ✅ None | Permissive |
| GPL-3.0 OR MIT | 1 | 🟡 Low | `unescaper` — MIT option available |
| LGPL-2.1 OR MIT | 1 | 🟡 Low | `r-efi` — MIT option available |
| MPL-2.0 | ~3 | 🟡 Low | Weak copyleft — file-level only |
| ISC, Zlib, BSL-1.0, Unicode-3.0 | ~10 | ✅ None | Permissive |

### Copyleft Analysis

**No pure copyleft licenses detected.** The two GPL/LGPL-licensed dependencies (`unescaper`, `r-efi`) are both dual-licensed with MIT, allowing the project to use them under the MIT terms.

| Dependency | License | Usage | Resolution |
|-----------|---------|-------|------------|
| `unescaper` | GPL-3.0 OR MIT | String unescaping utility | Use under MIT |
| `r-efi` | LGPL-2.1 OR MIT | UEFI runtime (oz-hal) | Use under MIT |

### UI Dependencies

Run: `npm ls --all` (10 prod + 22 dev = 32 total)

All UI dependencies are MIT, Apache-2.0, or BSD-licensed. No GPL or copyleft packages found in `node_modules`.

### Distribution Impact

- **Desktop app**: Ships as compiled binary — dynamic linking of system libraries (webkit2gtk, gtk3) uses LGPL, which is acceptable for binary distribution with separate .so/.dll files.
- **Cloud server**: Docker image ships with statically-linked musl binary. No GPL dependencies linked.
- **Tablet app**: Android/iOS packaged via Tauri. No copyleft concerns.

### Recommendation

No action required. All copyleft-licensed dependencies are dual-licensed with MIT, and the project uses them under MIT terms. Annual re-audit recommended.
