# OZ-POS Packaging

This directory contains packaging metadata for platform-specific installers.

## Structure

```
packaging/
├── README.md                    ← This file
├── linux/
│   ├── oz-pos.desktop           ← Freedesktop .desktop entry
│   └── deb/
│       ├── postinst             ← Post-installation script
│       └── prerm                ← Pre-removal script
```

## Build

Package builds are handled by Tauri's bundler during `cargo tauri build`:

```bash
# All targets
cargo tauri build --bundles deb,appimage,msi,nsis,dmg

# Linux only
cargo tauri build --bundles deb,appimage

# Windows only (cross-compile from Windows host)
cargo tauri build --bundles msi,nsis

# macOS only
cargo tauri build --bundles dmg
```

## Platform-specific notes

### Windows (MSI / NSIS)
- MSI is built via WiX Toolset (requires `wix` on PATH or Tauri's bundled WiX)
- NSIS installer is built via NSIS (requires `makensis` on PATH or Tauri's bundled NSIS)
- Both installers embed the `icon.ico` from `src-tauri/icons/`
- Auto-update on Windows uses the MSI or NSIS installer (configured in `tauri.conf.json`)

### Linux (.deb / AppImage)
- `.deb` packages are built with `cargo-deb` (bundled with Tauri CLI)
- AppImage is built with `appimagetool` (bundled with Tauri CLI)
- Desktop file is installed to `/usr/share/applications/oz-pos.desktop`
- Icons are installed to `/usr/share/icons/hicolor/`
- Database and config data lives at `/var/lib/oz-pos/`

### macOS (DMG)
- DMG is built with `create-dmg` (bundled with Tauri CLI)
- Code signing requires an Apple Developer ID certificate

## CI/CD

The release workflow (`.github/workflows/release.yml`) automates building all
platform packages on tag push and uploading them to a GitHub Release draft.

## Auto-update

OZ-POS uses Tauri's built-in updater plugin. When a new release is published,
the app checks for updates on launch and prompts the user to install.

The updater signing key pair was generated with:

```bash
cargo tauri signer generate -w oz-pos-updater.key
```

**IMPORTANT**: The private key (`oz-pos-updater.key`) must be kept secret.
It is used to sign update metadata so that users can verify the authenticity
of updates. The public key (`oz-pos-updater.key.pub`) is embedded in
`tauri.conf.json`.
