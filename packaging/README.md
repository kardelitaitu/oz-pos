# OZ-POS Packaging

Platform installer metadata for Tauri bundler output.

## Structure

```
packaging/
├── linux/
│   ├── oz-pos.desktop    # Freedesktop .desktop entry
│   └── deb/
│       ├── postinst      # Post-install script
│       └── prerm         # Pre-removal script
├── mobile/               # Tauri mobile build guide for Android tablets and iPads
```

Package builds are handled by Tauri's bundler during `cargo tauri build`:

```bash
cargo tauri build --bundles deb,appimage,msi,nsis,dmg
```

## Platform notes

| Platform | Format | Requirements |
|----------|--------|-------------|
| Windows | MSI / NSIS | WiX / NSIS on PATH (or Tauri-bundled) |
| Linux | .deb / AppImage | Bundled with Tauri CLI |
| macOS | DMG | Code signing: Apple Developer ID cert |

- DB and config: `/var/lib/oz-pos/` (Linux), app data dir (Windows/macOS)
- Auto-update uses Tauri's updater plugin; signing key pair in `oz-pos-updater.key` / `.key.pub`
- Release workflow in `.github/workflows/release.yml` builds all platforms on tag push

> last audited 28-06-26 by docs-auditor
